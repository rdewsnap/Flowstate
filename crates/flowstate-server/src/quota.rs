//! Plans, metering and monthly quotas for hosted model calls.
//!
//! Usage is billed to one team per user: a pooled (team-plan) team they're
//! on if there is one, otherwise their personal team. Periods are calendar
//! months in UTC.

use serde::Serialize;
use serde_json::json;
use time::{Date, Month, OffsetDateTime, Time};

use crate::args;
use crate::db::{now_ms, Db};
use crate::error::{ApiError, ApiResult};

#[derive(Clone, Debug, Serialize)]
pub struct Billing {
    pub team_id: String,
    pub plan: String,
    pub plan_name: String,
    pub pooled: bool,
    pub seats: i64,
    pub requests_used: i64,
    pub requests_limit: i64,
    pub tokens_used: i64,
    pub tokens_limit: i64,
    pub period_start: i64,
    pub resets_at: i64,
}

/// Start of this UTC month and of the next, in Unix milliseconds.
pub fn month_bounds(now_ms: i64) -> (i64, i64) {
    let now = OffsetDateTime::from_unix_timestamp(now_ms.div_euclid(1000)).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let start = Date::from_calendar_date(now.year(), now.month(), 1).expect("first of month");
    let (year, month) = if now.month() == Month::December { (now.year() + 1, Month::January) } else { (now.year(), now.month().next()) };
    let next = Date::from_calendar_date(year, month, 1).expect("first of month");
    let ms = |d: Date| d.with_time(Time::MIDNIGHT).assume_utc().unix_timestamp() * 1000;
    (ms(start), ms(next))
}

const PLAN_COLS: &str = "t.id, p.id, p.name, p.monthly_requests, p.monthly_tokens, p.pooled";

pub async fn billing_for(db: &Db, user_id: &str) -> ApiResult<Billing> {
    let pooled = db
        .query_opt(
            &format!(
                "SELECT {PLAN_COLS} FROM memberships m JOIN teams t ON t.id = m.team_id JOIN plans p ON p.id = t.plan
                 WHERE m.user_id = $1 AND p.pooled = 1 AND p.monthly_requests > 0 ORDER BY m.created_at LIMIT 1"
            ),
            args![user_id],
        )
        .await?;
    let row = match pooled {
        Some(r) => r,
        None => db
            .query_opt(
                &format!(
                    "SELECT {PLAN_COLS} FROM memberships m JOIN teams t ON t.id = m.team_id JOIN plans p ON p.id = t.plan
                     WHERE m.user_id = $1 AND t.personal = 1 LIMIT 1"
                ),
                args![user_id],
            )
            .await?
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("user {user_id} has no personal team or its plan is missing")))?,
    };
    let team_id = row.text(0);
    let pooled = row.bool(5);
    let seats = if pooled {
        db.query_one("SELECT COUNT(*) FROM memberships WHERE team_id = $1", args![&team_id]).await?.int(0).max(1)
    } else {
        1
    };
    let (period_start, resets_at) = month_bounds(now_ms());
    let used = db
        .query_one(
            "SELECT COUNT(*), CAST(COALESCE(SUM(input_tokens + output_tokens), 0) AS BIGINT) FROM usage
             WHERE team_id = $1 AND created_at >= $2",
            args![&team_id, period_start],
        )
        .await?;
    Ok(Billing {
        team_id,
        plan: row.text(1),
        plan_name: row.text(2),
        pooled,
        seats,
        requests_used: used.int(0),
        requests_limit: row.int(3) * seats,
        tokens_used: used.int(1),
        tokens_limit: row.int(4) * seats,
        period_start,
        resets_at,
    })
}

/// The caller's billing, or 402 if this month's allowance is used up.
pub async fn check(db: &Db, user_id: &str) -> ApiResult<Billing> {
    let b = billing_for(db, user_id).await?;
    let message = if b.requests_limit == 0 {
        format!("Hosted cleanup isn't included in the {} plan. Local cleanup on your Mac is unaffected.", b.plan_name)
    } else if b.requests_used >= b.requests_limit || b.tokens_used >= b.tokens_limit {
        format!("This month's hosted cleanup allowance for the {} plan is used up. Local cleanup on your Mac is unaffected.", b.plan_name)
    } else {
        return Ok(b);
    };
    Err(ApiError::QuotaExceeded {
        message,
        details: json!({
            "plan": b.plan,
            "requests_used": b.requests_used,
            "requests_limit": b.requests_limit,
            "tokens_used": b.tokens_used,
            "tokens_limit": b.tokens_limit,
            "resets_at": b.resets_at,
        }),
    })
}

pub async fn record(
    db: &Db,
    user_id: &str,
    team_id: &str,
    kind: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
) -> ApiResult<()> {
    db.execute(
        "INSERT INTO usage (user_id, team_id, kind, model, input_tokens, output_tokens, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        args![user_id, team_id, kind, model, input_tokens, output_tokens, now_ms()],
    )
    .await?;
    Ok(())
}

/// Set the plan of a team, or of a user's personal team when `target` is
/// an email address. Used by the `set-plan` command.
pub async fn set_plan(db: &Db, target: &str, plan: &str) -> anyhow::Result<String> {
    if db.query_opt("SELECT 1 FROM plans WHERE id = $1", args![plan]).await?.is_none() {
        anyhow::bail!("no plan named {plan:?}");
    }
    let team = if target.contains('@') {
        db.query_opt(
            "SELECT t.id FROM teams t JOIN users u ON u.id = t.created_by WHERE u.email = $1 AND t.personal = 1",
            args![target.trim().to_lowercase()],
        )
        .await?
    } else {
        db.query_opt("SELECT id FROM teams WHERE id = $1", args![target]).await?
    };
    let team_id = team.ok_or_else(|| anyhow::anyhow!("no user or team matches {target:?}"))?.text(0);
    db.execute("UPDATE teams SET plan = $1 WHERE id = $2", args![plan, &team_id]).await?;
    Ok(team_id)
}

#[cfg(test)]
mod tests {
    use super::month_bounds;

    #[test]
    fn month_bounds_roll_over_the_year() {
        // 2026-12-15T12:00:00Z
        let (start, next) = month_bounds(1_797_336_000_000);
        assert_eq!(start, 1_796_083_200_000); // 2026-12-01
        assert_eq!(next, 1_798_761_600_000); // 2027-01-01
    }
}

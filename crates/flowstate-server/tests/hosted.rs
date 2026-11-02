mod common;

use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use common::{team_with_member, TestApp};
use flowstate_server::config::{LlmConfig, LlmProvider};
use flowstate_server::{args, quota, Config};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<(HeaderMap, Value)>>>;

/// A stand-in for the Messages API that records what it was sent.
async fn fake_anthropic() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let app = Router::new().route(
        "/v1/messages",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let log = log.clone();
            async move {
                let summarizing = body["system"].as_str().unwrap_or("").contains("summarize meeting transcripts");
                log.lock().unwrap().push((headers, body));
                let text = if summarizing {
                    "{\"summary_md\": \"Agreed to ship Friday.\", \"action_items\": [{\"text\": \"Tag the release\", \"owner\": \"Bo\", \"due\": null}]}"
                } else {
                    "Send it to Priya on Friday."
                };
                Json(json!({
                    "type": "message", "role": "assistant", "stop_reason": "end_turn",
                    "content": [{ "type": "text", "text": text }],
                    "usage": { "input_tokens": 120, "output_tokens": 30 },
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

async fn app_with_llm(base_url: &str) -> TestApp {
    let mut config = Config::for_tests();
    config.llm = Some(LlmConfig {
        provider: LlmProvider::Anthropic,
        base_url: base_url.to_string(),
        api_key: "sk-test".into(),
        model: "claude-haiku-5-5".into(),
        summary_model: "claude-haiku-5-5".into(),
    });
    TestApp::with_config(config).await
}

fn dictation() -> Value {
    json!({
        "transcript": "send it to sam on friday actually priya instead",
        "app_name": "Slack", "style": "Casual chat.", "vocabulary": ["Priya"],
    })
}

#[tokio::test]
async fn free_plan_gets_402_without_calling_upstream() {
    let (url, seen) = fake_anthropic().await;
    let app = app_with_llm(&url).await;
    let token = app.sign_in("free@example.com").await;

    let (status, body) = app.post("/v1/cleanup", &token, dictation()).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(body["error"]["code"], "quota_exceeded");
    assert_eq!(body["error"]["plan"], "free");
    assert_eq!(body["error"]["requests_limit"], 0);
    assert!(body["error"]["message"].as_str().unwrap().contains("Local cleanup"));
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn pro_plan_cleans_up_and_meters_usage_until_the_limit() {
    let (url, seen) = fake_anthropic().await;
    let app = app_with_llm(&url).await;
    let token = app.sign_in("pro@example.com").await;
    quota::set_plan(&app.state.db, "pro@example.com", "pro").await.unwrap();

    let (status, body) = app.post("/v1/cleanup", &token, dictation()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["text"], "Send it to Priya on Friday.");
    assert_eq!(body["usage"]["input_tokens"], 120);

    {
        let seen = seen.lock().unwrap();
        let (headers, sent) = &seen[0];
        assert_eq!(headers["anthropic-version"], "2026-10-21");
        assert_eq!(headers["x-api-key"], "sk-test");
        assert_eq!(sent["model"], "claude-haiku-5-5");
        assert!(sent["system"].as_str().unwrap().contains("Resolve self-corrections"));
        let user = sent["messages"][0]["content"].as_str().unwrap();
        assert!(user.contains("<vocabulary>Priya</vocabulary>"));
        assert!(user.ends_with("<transcript>send it to sam on friday actually priya instead</transcript>"));
        assert!(sent.get("temperature").is_none());
    }

    let (_, usage) = app.get("/v1/usage", &token).await;
    assert_eq!(usage["plan"], "pro");
    assert_eq!(usage["requests_used"], 1);
    assert_eq!(usage["tokens_used"], 150);

    // Shrink the plan so the next request is over the limit.
    app.state
        .db
        .execute("UPDATE plans SET monthly_requests = $1 WHERE id = 'pro'", args![1_i64])
        .await
        .unwrap();
    let (status, body) = app.post("/v1/cleanup", &token, dictation()).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(body["error"]["requests_used"], 1);
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn team_plan_pools_usage_across_members() {
    let (url, _) = fake_anthropic().await;
    let app = app_with_llm(&url).await;
    let (owner, member, team_id) = team_with_member(&app, "pool-a@example.com", "pool-b@example.com").await;
    quota::set_plan(&app.state.db, &team_id, "team").await.unwrap();
    // One request per seat, two seats: two requests for the whole team.
    app.state.db.execute("UPDATE plans SET monthly_requests = 1 WHERE id = 'team'", args![]).await.unwrap();

    assert_eq!(app.post("/v1/cleanup", &owner, dictation()).await.0, StatusCode::OK);
    assert_eq!(app.post("/v1/cleanup", &member, dictation()).await.0, StatusCode::OK);
    let (status, body) = app.post("/v1/cleanup", &owner, dictation()).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(body["error"]["requests_limit"], 2);
    let (_, usage) = app.get("/v1/usage", &member).await;
    assert_eq!(usage["team_id"], team_id.as_str());
    assert_eq!(usage["seats"], 2);
}

#[tokio::test]
async fn summarize_returns_summary_and_action_items() {
    let (url, _) = fake_anthropic().await;
    let app = app_with_llm(&url).await;
    let token = app.sign_in("sum@example.com").await;
    quota::set_plan(&app.state.db, "sum@example.com", "pro").await.unwrap();

    let (status, body) = app
        .post(
            "/v1/summarize",
            &token,
            json!({ "title": "Release sync", "participants": ["Ada", "Bo"], "transcript": "Ada: ship Friday. Bo: I'll tag it." }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["summary_md"], "Agreed to ship Friday.");
    assert_eq!(body["action_items"][0]["owner"], "Bo");
}

#[tokio::test]
async fn hosted_cleanup_is_off_without_an_upstream() {
    let app = TestApp::new().await;
    let token = app.sign_in("nollm@example.com").await;
    let (status, _) = app.post("/v1/cleanup", &token, dictation()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

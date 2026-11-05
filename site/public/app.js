// Data from training/RESULTS.md. Keep this in sync when results change.
const MODELS = [
  { key: "4b",   name: "Qwen3-4B (baseline)",     color: "var(--s1)" },
  { key: "1.7b", name: "fnspeak-cleanup 1.7B",  color: "var(--s2)" },
  { key: "0.6b", name: "fnspeak-cleanup 0.6B",  color: "var(--s3)" },
];
const SETS = {
  tts:   { label: "11 TTS clips, Whisper", clips: 11, rows: {
    "4b":   { wer: 1.1, terms: 100, fillers: 0, repair: 3.3, lat: 301, fb: 0 },
    "1.7b": { wer: 2.2, terms: 100, fillers: 0, repair: 3.3, lat: 167, fb: 0 },
    "0.6b": { wer: 3.4, terms: 100, fillers: 0, repair: 6.7, lat: 90,  fb: 0 },
  }},
  valid: { label: "200 validation pairs, cleanup only", clips: 200, rows: {
    "4b":   { wer: 14.8, terms: 62.5, fillers: 18, repair: 29.1, lat: 652, fb: 13 },
    "1.7b": { wer: 4.1,  terms: 87.5, fillers: 2,  repair: 7.5,  lat: 315, fb: 0 },
    "0.6b": { wer: 5.2,  terms: 87.5, fillers: 4,  repair: 7.6,  lat: 170, fb: 0 },
  }},
};
const DATA_SOURCES = [
  { name: "corrupt.py (WikiText-2, oasst1, Dolly)", n: 11000 },
  { name: "SottoASR", n: 6000 },
  { name: "Aawaaz", n: 4000 },
  { name: "Disfl-QA", n: 2500 },
  { name: "generate_local.py with the 4B", n: 2197 },
];

const $ = (s) => document.querySelector(s);
const svgNS = "http://www.w3.org/2000/svg";
const el = (tag, attrs = {}, text) => {
  const n = document.createElementNS(svgNS, tag);
  for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
  if (text != null) n.textContent = text;
  return n;
};
const h = (tag, props = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) k === "class" ? (n.className = v) : (n[k] = v);
  n.append(...kids);
  return n;
};

// Tooltip
const tip = h("div", { class: "tip" });
document.body.appendChild(tip);
function bindTip(node, text) {
  node.addEventListener("mousemove", (e) => { tip.textContent = text; tip.style.left = e.clientX + "px"; tip.style.top = e.clientY + "px"; tip.classList.add("on"); });
  node.addEventListener("mouseleave", () => tip.classList.remove("on"));
}

// Horizontal bars, one per model, fixed order and fixed colors.
function barChart(container, metric, unit, max) {
  const set = SETS[currentSet];
  const W = 460, rowH = 34, padL = 8, padR = 56, H = rowH * MODELS.length + 28;
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": metric });
  const scale = (v) => padL + (v / max) * (W - padL - padR);
  const ticks = 4;
  for (let i = 0; i <= ticks; i++) {
    const v = (max / ticks) * i, x = scale(v);
    svg.appendChild(el("line", { x1: x, x2: x, y1: 0, y2: rowH * MODELS.length, class: "grid" }));
    svg.appendChild(el("text", { x, y: rowH * MODELS.length + 16, "text-anchor": i === 0 ? "start" : "middle" }, fmt(v) + (i === ticks ? unit : "")));
  }
  svg.appendChild(el("line", { x1: padL, x2: padL, y1: 0, y2: rowH * MODELS.length, class: "axis" }));
  MODELS.forEach((m, i) => {
    const v = set.rows[m.key][metric];
    const y = i * rowH + 6, hh = rowH - 12, w = Math.max(scale(v) - padL, 0);
    const bar = el("rect", { x: padL, y, width: w, height: hh, fill: m.color, class: "bar" });
    const cap = el("rect", { x: padL + w - Math.min(4, w), y, width: Math.min(4, w), height: hh, rx: 4, fill: m.color, class: "bar" });
    svg.append(bar, cap, el("text", { x: padL + w + 8, y: y + hh / 2 + 4, class: "val" }, fmt(v) + unit));
    bindTip(bar, `${m.name}: ${fmt(v)}${unit}`);
    bindTip(cap, `${m.name}: ${fmt(v)}${unit}`);
  });
  container.replaceChildren(svg);
}

function scatter(container) {
  const W = 460, H = 200, pad = { l: 40, r: 16, t: 10, b: 30 };
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": "Latency vs word error rate" });
  const xs = (v) => pad.l + (v / 700) * (W - pad.l - pad.r);
  const ys = (v) => H - pad.b - (v / 16) * (H - pad.t - pad.b);
  for (let i = 0; i <= 4; i++) {
    const y = ys(i * 4);
    svg.appendChild(el("line", { x1: pad.l, x2: W - pad.r, y1: y, y2: y, class: "grid" }));
    svg.appendChild(el("text", { x: pad.l - 6, y: y + 4, "text-anchor": "end" }, i * 4 + "%"));
  }
  for (let i = 0; i <= 7; i++) {
    svg.appendChild(el("text", { x: xs(i * 100), y: H - pad.b + 16, "text-anchor": "middle" }, i * 100 + (i === 7 ? " ms" : "")));
  }
  svg.appendChild(el("line", { x1: pad.l, x2: W - pad.r, y1: ys(0), y2: ys(0), class: "axis" }));
  for (const [sk, set] of Object.entries(SETS)) {
    MODELS.forEach((m) => {
      const r = set.rows[m.key];
      const cx = xs(r.lat), cy = ys(r.wer);
      const g = el("g", { class: "dot" });
      g.appendChild(el("circle", { cx, cy, r: 7, class: "ring" }));
      g.appendChild(sk === "tts"
        ? el("circle", { cx, cy, r: 5.5, fill: m.color })
        : el("rect", { x: cx - 5, y: cy - 5, width: 10, height: 10, rx: 2, fill: m.color }));
      bindTip(g, `${m.name} · ${set.label}: ${r.wer}% WER, ${r.lat} ms`);
      svg.appendChild(g);
    });
  }
  svg.appendChild(el("text", { x: pad.l + 6, y: pad.t + 12 }, "● TTS clips   ■ validation pairs"));
  container.replaceChildren(svg);
}

function dataChart(container) {
  const total = DATA_SOURCES.reduce((a, s) => a + s.n, 0);
  const W = 960, H = 72;
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": "Training data sources" });
  let x = 0;
  const steps = [1, 0.78, 0.58, 0.42, 0.28]; // one neutral ramp (ink at falling opacity), ordered by size, works in both themes
  DATA_SOURCES.forEach((s, i) => {
    const w = (s.n / total) * W - 2;
    const r = el("rect", { x, y: 0, width: w, height: 26, fill: "var(--ink)", "fill-opacity": steps[i], rx: 2 });
    bindTip(r, `${s.name}: ${s.n.toLocaleString()} pairs`);
    svg.append(r, el("text", { x, y: 46, class: "val" }, s.n.toLocaleString()), el("text", { x, y: 62 }, shorten(s.name, w)));
    x += w + 2;
  });
  container.replaceChildren(svg);
}
function shorten(s, w) { const maxChars = Math.floor(w / 6.4); return s.length > maxChars ? s.slice(0, Math.max(maxChars - 1, 3)) + "…" : s; }

function legend() {
  $("#legend").replaceChildren(...MODELS.map((m) => {
    const i = h("i"); i.style.background = m.color;
    return h("span", {}, i, m.name);
  }));
}

function table() {
  const heads = ["Model", "Set", "Clips", "WER", "Terms right", "Fillers left", "Repair WER", "Latency", "Fallbacks"];
  const thead = h("thead", {}, h("tr", {}, ...heads.map((t) => h("th", {}, t))));
  const tbody = h("tbody");
  for (const set of Object.values(SETS)) MODELS.forEach((m) => {
    const r = set.rows[m.key];
    tbody.appendChild(h("tr", {}, ...[m.name, set.label, set.clips, r.wer + "%", r.terms + "%", r.fillers, r.repair + "%", r.lat + " ms", r.fb].map((v) => h("td", {}, String(v)))));
  });
  $("#results").replaceChildren(thead, tbody);
}

function fmt(v) { return Number.isInteger(v) ? String(v) : v.toFixed(1); }

let currentSet = "tts";
function render() {
  const tts = currentSet === "tts";
  barChart($("#c-wer"), "wer", "%", tts ? 4 : 16);
  barChart($("#c-repair"), "repair", "%", tts ? 8 : 32);
  barChart($("#c-lat"), "lat", " ms", tts ? 400 : 800);
}
document.querySelectorAll(".seg").forEach((b) => b.addEventListener("click", () => {
  document.querySelectorAll(".seg").forEach((x) => { x.classList.remove("active"); x.setAttribute("aria-selected", "false"); });
  b.classList.add("active"); b.setAttribute("aria-selected", "true");
  currentSet = b.dataset.set; render();
}));
render(); scatter($("#c-scatter")); dataChart($("#c-data")); legend(); table();

// Latest release from GitHub; falls back to the baked-in release if offline or rate-limited.
(async () => {
  try {
    const r = await fetch("https://api.github.com/repos/rdewsnap/FnSpeak/releases/latest", { headers: { Accept: "application/vnd.github+json" } });
    if (!r.ok) return;
    const rel = await r.json();
    const assets = Array.isArray(rel.assets) ? rel.assets : [];
    const dmg = assets.find((a) => /\.dmg$/.test(a.name));
    $("#rel-tag").textContent = rel.tag_name;
    $("#rel-date").textContent = "Published " + new Date(rel.published_at).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });
    if (dmg) { $("#dl-primary").href = dmg.browser_download_url; $("#dl-meta").textContent = `${rel.tag_name} · Apple Silicon · macOS 14+ · ${mb(dmg.size)}`; }
    const kind = (n) => /\.dmg$/.test(n) ? "Disk image" : /\.zip$/.test(n) ? "App bundle" : /^SHA256/.test(n) ? "Checksums" : "";
    $("#assets tbody").replaceChildren(...assets.map((a) =>
      h("tr", {}, h("td", {}, h("a", { href: a.browser_download_url, textContent: a.name })), h("td", {}, mb(a.size)), h("td", { class: "muted" }, kind(a.name)))));
  } catch (_) { /* keep static fallback */ }
})();
function mb(b) { return b < 1024 ? b + " B" : b < 1048576 ? Math.round(b / 1024) + " KB" : Math.round(b / 1048576) + " MB"; }

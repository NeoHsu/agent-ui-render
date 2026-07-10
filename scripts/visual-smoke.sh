#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="$ROOT/target/visual-smoke"
mkdir -p "$OUT"

if [[ ! -s "$ROOT/generated/renderer.js" || ! -s "$ROOT/generated/renderer.css" ]]; then
	(cd "$ROOT/renderer-vue" && bun run build)
fi

python3 - "$ROOT" "$OUT" <<'PY'
from __future__ import annotations

import copy
import html
import itertools
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
css = (root / "generated" / "renderer.css").read_text()
js = (root / "generated" / "renderer.js").read_text()

themes = ["report-light", "technical-dark", "executive-clean"]
densities = ["comfortable", "compact"]
emphases = ["strong", "subtle"]

base_payload = {
    "schema": "ui.input.normalized",
    "version": 1,
    "title": "Agent UI Visual Smoke: All Components and Styles",
    "summary": "Visual smoke fixture covering every renderer component, chart kind, alert level, semantic tone, table state, theme, density, and emphasis variant.",
    "datasets": {
        "timeline": {
            "columns": [
                {"key": "phase", "label": "Phase", "type": "string"},
                {"key": "minute", "label": "Minute", "type": "number"},
                {"key": "error_rate", "label": "Error Rate", "type": "percent"},
                {"key": "p95_latency", "label": "P95 Latency", "type": "number", "unit": "ms"},
                {"key": "throughput", "label": "Throughput", "type": "number"},
            ],
            "rows": [
                ["Baseline", 0, 0.004, 180, 980],
                ["Rollout", 10, 0.012, 260, 930],
                ["Degraded", 20, 0.034, 540, 710],
                ["Mitigating", 32, 0.019, 380, 840],
                ["Recovered", 42, 0.006, 210, 970],
            ],
        },
        "financial_quarters": {
            "columns": [
                {"key": "quarter", "label": "Quarter", "type": "string"},
                {"key": "revenue", "label": "Revenue", "type": "currency", "unit": "USD"},
                {"key": "profit", "label": "Profit", "type": "currency", "unit": "USD"},
            ],
            "rows": [
                ["Q1", 1030000, 260000],
                ["Q2", 1140000, 310000],
                ["Q3", 1210000, 340000],
                ["Q4", 1340000, 370000],
            ],
        },
        "impact_mix": {
            "columns": [
                {"key": "impact_area", "label": "Impact Area", "type": "string"},
                {"key": "requests", "label": "Requests", "type": "number"},
                {"key": "status", "label": "Status", "type": "string"},
            ],
            "rows": [
                ["API 5xx", 7600, "critical"],
                ["Checkout timeouts", 3200, "error"],
                ["Slow searches", 1450, "warning"],
                ["Background retries", 600, "recovering"],
                ["Recovered callbacks", 420, "success"],
            ],
        },
        "evidence": {
            "columns": [
                {"key": "source", "label": "Source", "type": "string"},
                {"key": "observation", "label": "Observation", "type": "string"},
                {"key": "confidence", "label": "Confidence", "type": "percent"},
                {"key": "status", "label": "Status", "type": "string"},
            ],
            "rows": [
                ["Deploy log", "Config rollout completed at 10:09 UTC", 0.88, "confirmed"],
                ["Metrics", "Error rate peaked near 10:20 UTC", 0.92, "supporting"],
                ["Trace sample", "Queue wait increased during rollout", 0.76, "warning"],
                ["Customer tickets", "Checkout reports started after rollout", 0.61, "error"],
                ["Database log", "FATAL runtime/datadir mismatch", 0.94, "critical"],
                ["Backfill", "Signal unavailable for one shard", None, "neutral"],
            ],
        },
        "actions": {
            "columns": [
                {"key": "action", "label": "Action", "type": "string"},
                {"key": "owner", "label": "Owner", "type": "string"},
                {"key": "priority", "label": "Priority", "type": "string"},
                {"key": "due", "label": "Due", "type": "date"},
                {"key": "status", "label": "Status", "type": "string"},
            ],
            "rows": [
                ["Add rollout guardrail", "Platform", "High", "2026-07-16", "planned"],
                ["Add saturation alert", "SRE", "High", "2026-07-14", "in progress"],
                ["Document rollback criteria", "API", "Medium", "2026-07-18", "pending"],
                ["Verify compatibility", "Database", "High", "2026-07-13", "blocked"],
            ],
        },
        "empty_records": {
            "columns": [
                {"key": "item", "label": "Item", "type": "string"},
                {"key": "status", "label": "Status", "type": "string"},
            ],
            "rows": [],
        },
    },
    "metrics": [
        {"label": "Number Metric", "value": 12850, "format": "number", "delta": {"value": 0.12, "format": "percent", "direction": "up", "label": "+12% vs baseline"}},
        {"label": "Currency Metric", "value": 42000, "format": "currency", "unit": "USD", "delta": {"value": -0.05, "format": "percent", "direction": "down", "label": "-5% vs plan"}},
        {"label": "Percent Metric", "value": 0.82, "format": "percent", "delta": {"value": 0, "format": "percent", "direction": "flat", "label": "flat"}},
        {"label": "String Metric", "value": "Ready", "format": "string"},
    ],
    "insights": [
        "Insight list style with primary marker color.",
        "Second insight validates multi-row spacing and text rhythm.",
    ],
    "markdown": [
        {
            "title": "Markdown Semantic Tone Styles",
            "content": "### Executive summary\nThe renderer supports **strong**, *emphasis*, `inline code`, and safe links like [guide](https://example.com/report-guide).\n\n> Semantic tones: {critical: critical}, {error: error}, {warning: warning}, {success: success}, {info: info}, and {muted: muted}.\n\n- unordered item\n- another item\n\n1. ordered step\n2. follow-up step\n\n---\n\n```sql\nselect phase, error_rate from service_metrics;\n```",
        },
        {
            "title": "Narrative Block",
            "content": "Use markdown for prose only; records and charts remain structured datasets. {success: Confidence high} with {warning: pending verification}.",
        },
    ],
    "views": [
        {"intent": "trend", "data": "timeline", "x": "phase", "measures": ["error_rate", "p95_latency"], "priority": "high", "title": "Line Chart: Trend"},
        {"intent": "relationship", "data": "timeline", "x": "minute", "measures": ["p95_latency"], "priority": "medium", "title": "Scatter Chart: Relationship"},
        {"intent": "composition", "data": "impact_mix", "x": "impact_area", "measures": ["requests"], "priority": "high", "title": "Pie Chart: Composition"},
        {"intent": "comparison", "data": "financial_quarters", "x": "quarter", "measures": ["revenue", "profit"], "priority": "high", "title": "Vertical Grouped Bar: Period Comparison"},
        {"intent": "comparison", "data": "impact_mix", "x": "impact_area", "measures": ["requests"], "priority": "medium", "title": "Horizontal Bar: Category Comparison"},
        {"intent": "distribution", "data": "impact_mix", "x": "impact_area", "measures": ["requests"], "priority": "low", "title": "Bar Chart: Distribution"},
        {"intent": "precise_records", "data": "evidence", "priority": "medium", "title": "Table: Status Badges"},
        {"intent": "precise_records", "data": "actions", "columns": ["action", "priority", "status"], "priority": "high", "title": "Table: Projected Columns"},
        {"intent": "precise_records", "data": "empty_records", "priority": "low", "title": "Table: Empty State"},
    ],
    "alerts": [
        {"level": "info", "title": "Info alert", "content": "Informational alert style."},
        {"level": "success", "title": "Success alert", "content": "Success alert style."},
        {"level": "warning", "title": "Warning alert", "content": "Warning alert style."},
        {"level": "error", "title": "Error alert", "content": "Error alert style."},
        {"level": "critical", "title": "Critical alert", "content": "Critical alert style."},
    ],
    "assumptions": [
        "Assumption block validates muted card styling.",
        "Payload text is treated as data and escaped by renderers.",
    ],
}

assert {alert["level"] for alert in base_payload["alerts"]} == {"info", "success", "warning", "error", "critical"}
assert {view["intent"] for view in base_payload["views"]} == {"trend", "relationship", "composition", "comparison", "distribution", "precise_records"}
assert {metric.get("format") for metric in base_payload["metrics"]} == {"number", "currency", "percent", "string"}
assert base_payload["insights"] and base_payload["assumptions"] and base_payload["markdown"]

variant_files = []
for theme, density, emphasis in itertools.product(themes, densities, emphases):
    payload = copy.deepcopy(base_payload)
    payload["theme"] = theme
    payload["density"] = density
    payload["emphasis"] = emphasis
    payload["title"] = f"Agent UI Visual Smoke: {theme} / {density} / {emphasis}"
    stem = f"all-components-{theme}-{density}-{emphasis}"
    json_path = out / f"{stem}.normalized.json"
    html_path = out / f"{stem}.html"
    json_text = json.dumps(payload, ensure_ascii=False, indent=2)
    json_path.write_text(json_text + "\n")
    embedded_payload = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
    html_doc = f"""<!doctype html>
<html lang=\"zh-Hant\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>{html.escape(payload['title'])}</title>
<style>{css}</style>
</head>
<body class=\"agent-ui-standalone\" data-theme=\"{html.escape(theme)}\">
<div id=\"agent-ui-root\"></div>
<script type=\"application/json\" id=\"agent-ui-payload\">{embedded_payload}</script>
<script>{js}</script>
</body>
</html>
"""
    html_path.write_text(html_doc, encoding="utf-8")
    variant_files.append((theme, density, emphasis, html_path.name, html_doc))

index_cards = "\n".join(
    f"""
    <section class=\"variant-card\">
      <h2>{html.escape(theme)} / {html.escape(density)} / {html.escape(emphasis)}</h2>
      <p><a href=\"{html.escape(file_name)}\">Open full preview</a></p>
      <iframe srcdoc=\"{html.escape(html_doc, quote=True)}\" loading=\"lazy\"></iframe>
    </section>
    """
    for theme, density, emphasis, file_name, html_doc in variant_files
)
(out / "index.html").write_text(f"""<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>Agent UI Visual Smoke Index</title>
<style>
body {{ margin: 0; padding: 24px; background: #111827; color: #f9fafb; font-family: ui-sans-serif, system-ui, sans-serif; }}
a {{ color: #93c5fd; }}
.grid {{ display: grid; gap: 24px; }}
.variant-card {{ border: 1px solid #374151; border-radius: 16px; padding: 16px; background: #1f2937; }}
.variant-card h2 {{ margin: 0 0 8px; font-size: 1rem; }}
.variant-card iframe {{ width: 100%; height: 980px; border: 1px solid #4b5563; border-radius: 12px; background: white; }}
</style>
</head>
<body>
<h1>Agent UI Visual Smoke Index</h1>
<p>Each iframe renders every governed component and style variant for one theme/density/emphasis combination.</p>
<div class=\"grid\">{index_cards}</div>
</body>
</html>
""", encoding="utf-8")

(out / "CHECKLIST.md").write_text("""# Agent UI Visual Smoke Checklist

Open `index.html` and verify every variant renders without console/runtime errors.

Coverage in every variant:

- Header, footer, alerts, metric grid, insight list, markdown cards, assumptions.
- Alert levels: info, success, warning, error, critical.
- Metric formats: number, currency, percent, string, including metric delta labels.
- Markdown syntax: headings, paragraph, blockquote, ordered/unordered lists, hr, fenced code, link, inline code, strong/emphasis.
- Semantic tones: critical, error, warning, success, info, muted.
- Charts: line, scatter, pie, vertical grouped bar, horizontal grouped bar, bar distribution.
- Tables: full records/status badges, projected columns, empty state.
- View priorities: high, medium, low.
- Themes: report-light, technical-dark, executive-clean.
- Density: comfortable, compact.
- Emphasis: strong, subtle.
""", encoding="utf-8")

print(f"visual smoke artifacts: {out}")
print(f"index: {out / 'index.html'}")
print(f"variants: {len(variant_files)}")
PY

for expected in index.html CHECKLIST.md; do
	test -s "$OUT/$expected"
done

grep -q 'Agent UI Visual Smoke Index' "$OUT/index.html"
grep -q 'all-components-technical-dark-compact-subtle.html' "$OUT/index.html"

echo "visual smoke artifacts OK: $OUT/index.html"

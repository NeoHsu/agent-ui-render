#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="$ROOT/target/release/agent-ui-render"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

cargo build --release --bin agent-ui-render

for schema in compact compact-v2 normalized normalized-v2 spec spec-v2 config; do
	"$BIN" schema print "$schema" >"$TMPDIR/$schema.schema.json"
	python3 -m json.tool "$TMPDIR/$schema.schema.json" >/dev/null
done

for input in examples/*.input.json; do
	"$BIN" validate "$input"
done

"$BIN" normalize examples/revenue-overview.input.json "$TMPDIR/revenue.normalized.json"
python3 -m json.tool "$TMPDIR/revenue.normalized.json" >/dev/null
"$BIN" plan examples/revenue-overview.input.json "$TMPDIR/revenue.spec.json"
python3 -m json.tool "$TMPDIR/revenue.spec.json" >/dev/null

"$BIN" render html examples/revenue-overview.input.json "$TMPDIR/revenue.html"
"$BIN" render static-html examples/revenue-overview.input.json "$TMPDIR/revenue.static.html"
grep -q 'agent-ui-root' "$TMPDIR/revenue.html"
grep -q 'agent-ui-payload' "$TMPDIR/revenue.html"
grep -q 'agent-ui-render' "$TMPDIR/revenue.static.html"
grep -q 'Revenue Overview' "$TMPDIR/revenue.static.html"

"$BIN" normalize examples/v2-chart-showcase.input.json "$TMPDIR/charts.normalized.json"
"$BIN" plan examples/v2-chart-showcase.input.json "$TMPDIR/charts.spec.json"
"$BIN" render html examples/v2-chart-showcase.input.json "$TMPDIR/charts.html"
"$BIN" render static-html examples/v2-chart-showcase.input.json "$TMPDIR/charts.static.html"
grep -q '"version":2' "$TMPDIR/charts.html"
grep -q 'vega-chart' "$TMPDIR/charts.html"
grep -q 'Interactive line chart' "$TMPDIR/charts.static.html"

echo "release verification OK"

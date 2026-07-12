#!/usr/bin/env bash
# Validate every complete compact payload example in skill and doc markdown
# against the real CLI. A "complete payload" is a fenced ```json block whose
# content is a JSON object containing a "version" key; tuple fragments and
# non-payload JSON (for example config examples) are skipped. The CLI is the
# source of truth; failing examples mean the markdown is stale.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

BIN=${AGENT_UI_RENDER_BIN:-}
if [[ -z "$BIN" ]]; then
  if command -v agent-ui-render >/dev/null 2>&1; then
    BIN=agent-ui-render
  elif [[ -x target/release/agent-ui-render ]]; then
    BIN=target/release/agent-ui-render
  else
    BIN="cargo run --quiet -p agent-ui-render --"
  fi
fi

FILES=(README.md docs/*.md skills/agent-ui-render/SKILL.md
  skills/agent-ui-render/references/*.md)

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# Split fenced ```json blocks into $TMPDIR/<n>.json plus a <n>.src sidecar
# recording the originating file and fence line.
awk -v outdir="$TMPDIR" '
  FNR == 1 { inblock = 0 }
  /^[[:space:]]*```json[[:space:]]*$/ && !inblock {
    inblock = 1
    count += 1
    out = outdir "/" count ".json"
    print FILENAME ":" FNR > (outdir "/" count ".src")
    next
  }
  /^[[:space:]]*```[[:space:]]*$/ && inblock {
    inblock = 0
    close(out)
    next
  }
  inblock { print > out }
' "${FILES[@]}"

failures=0
checked=0
shopt -s nullglob
for block in "$TMPDIR"/*.json; do
  src="$(cat "${block%.json}.src")"
  first_char="$(tr -d '[:space:]' <"$block" | head -c 1)"
  [[ "$first_char" == "{" ]] || continue
  grep -q '"version"' "$block" || continue
  checked=$((checked + 1))
  if ! output=$($BIN validate --warnings-as-errors "$block" 2>&1); then
    echo "FAIL: $src"
    echo "$output"
    failures=$((failures + 1))
  fi
done

if [[ $failures -ne 0 ]]; then
  echo "$failures markdown payload example(s) failed validation."
  exit 1
fi
echo "OK: $checked markdown payload example(s) validate cleanly."

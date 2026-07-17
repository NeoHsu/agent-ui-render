#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CHROME="${CHROME_BIN:-}"
if [[ -z "$CHROME" ]]; then
	for candidate in \
		google-chrome \
		google-chrome-stable \
		chromium \
		chromium-browser \
		"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"; do
		if command -v "$candidate" >/dev/null 2>&1; then
			CHROME="$(command -v "$candidate")"
			break
		fi
		if [[ -x "$candidate" ]]; then
			CHROME="$candidate"
			break
		fi
	done
fi
if [[ -z "$CHROME" ]]; then
	echo "interaction smoke skipped: Chrome/Chromium not found" >&2
	exit 2
fi

(cd renderer-vue && bun run build)
mkdir -p target/visual-smoke
INPUT="${INTERACTION_INPUT:-examples/v2-chart-showcase.input.json}"
HTML="${INTERACTION_HTML:-target/visual-smoke/v2-chart-showcase.html}"
CONFIG="${INTERACTION_CONFIG-scripts/fixtures/interaction.config.json}"
STARTUP_BUDGET_MS="${INTERACTION_STARTUP_BUDGET_MS:-8000}"
if [[ ! "$STARTUP_BUDGET_MS" =~ ^[1-9][0-9]*$ ]]; then
	echo "INTERACTION_STARTUP_BUDGET_MS must be a positive integer" >&2
	exit 2
fi
CONFIG_ARGS=()
if [[ -n "$CONFIG" ]]; then
	CONFIG_ARGS+=(--config "$CONFIG")
fi
cargo run --quiet -- "${CONFIG_ARGS[@]}" render html "$INPUT" "$HTML" >/dev/null

PORT="$({
	python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
} | tr -d '[:space:]')"
PROFILE="$(mktemp -d "${TMPDIR:-/tmp}/agent-ui-interaction-smoke.XXXXXX")"
LOG="$PROFILE/chrome.log"

"$CHROME" \
	--headless \
	--disable-gpu \
	--disable-dev-shm-usage \
	--no-sandbox \
	--allow-file-access-from-files \
	--remote-debugging-address=127.0.0.1 \
	--remote-debugging-port="$PORT" \
	--user-data-dir="$PROFILE/profile" \
	"file://$ROOT/$HTML" \
	>"$LOG" 2>&1 &
CHROME_PID=$!
# Invoked indirectly by the EXIT trap below.
# shellcheck disable=SC2329
cleanup() {
	kill "$CHROME_PID" 2>/dev/null || true
	wait "$CHROME_PID" 2>/dev/null || true
	rm -rf "$PROFILE" 2>/dev/null || true
}
trap cleanup EXIT

SCREENSHOT_ARGS=()
if [[ -n "${INTERACTION_SCREENSHOT_DIR:-}" ]]; then
	if [[ "$INTERACTION_SCREENSHOT_DIR" != "target/visual-smoke/interactions" ]]; then
		echo "INTERACTION_SCREENSHOT_DIR must be target/visual-smoke/interactions" >&2
		exit 2
	fi
	SCREENSHOT_ARGS+=("capture")
fi

for _ in $(seq 1 100); do
	if curl --silent --fail "http://127.0.0.1:$PORT/json" >/dev/null; then
		bun scripts/interaction-smoke.ts "$PORT" "$STARTUP_BUDGET_MS" "${SCREENSHOT_ARGS[@]}"
		exit 0
	fi
	sleep 0.2
done

echo "Chrome DevTools endpoint did not become ready" >&2
cat "$LOG" >&2
exit 1

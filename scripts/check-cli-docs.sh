#!/usr/bin/env bash
# Fail when docs/cli-reference.md no longer mentions a CLI command, render
# subcommand, or global flag that `--help` advertises. The --help output is the
# source of truth; this doc is a navigational overview of it.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

DOC=docs/cli-reference.md
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

missing=0

check_listed() {
  local token=$1
  if ! grep -q -- "$token" "$DOC"; then
    echo "MISSING in $DOC: $token"
    missing=1
  fi
}

help_commands() {
  $BIN "$@" --help | awk '/^Commands:/{f=1;next} /^$/{f=0} f{print $1}'
}

while read -r cmd; do
  [[ "$cmd" == "help" ]] && continue
  check_listed "$cmd"
done < <(help_commands)

while read -r cmd; do
  [[ "$cmd" == "help" ]] && continue
  check_listed "render $cmd"
done < <(help_commands render)

while read -r flag; do
  check_listed "$flag"
done < <($BIN --help | grep -oE -- '--[a-z][a-z-]+' | sort -u | grep -vE '^--(help|version)$')

if [[ $missing -ne 0 ]]; then
  echo "docs/cli-reference.md is stale relative to the CLI --help output."
  exit 1
fi
echo "OK: docs/cli-reference.md covers all CLI commands and global flags."

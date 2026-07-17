#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

command -v actionlint >/dev/null 2>&1 || {
	echo "actionlint not found. Install pinned tools with: mise install" >&2
	exit 127
}

actionlint .github/workflows/*.yml

unpinned="$({
	grep -HnE '^[[:space:]]*(- )?uses:' .github/workflows/*.yml || true
} | grep -Ev '@[0-9a-f]{40}([[:space:]]|$)' || true)"
if [[ -n "$unpinned" ]]; then
	echo "GitHub Actions must use immutable 40-character commit SHAs:" >&2
	echo "$unpinned" >&2
	exit 1
fi

echo "OK: workflow syntax is valid and all Actions are pinned."

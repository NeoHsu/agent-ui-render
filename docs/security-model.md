# Security Model

`agent-ui-render` treats all payload strings as untrusted data.

## Trust boundaries

- Untrusted: compact JSON payloads produced by models or tools.
- Trusted: CLI binary, bundled renderer assets, schemas, and optional host config.
- Host config controls runtime limits and optional renderer theme color tokens;
  payloads cannot raise their own limits or provide CSS.

## What is allowed

Payloads may contain structured data, primitive table cells, metrics, alerts, and
safe markdown text. Markdown is parsed as a small allowlist subset:

- headings `#` through `###`
- paragraphs, blockquotes, unordered and ordered lists
- fenced code blocks
- emphasis/strong/inline code
- safe links: `https://`, `http://`, `mailto:`, `/path`, and `#anchor`
- semantic tokens like `{warning: pending}`

Raw HTML, Vue, React, SVG, CSS, event handlers, component/action injection, and
`javascript:` URLs are not part of the contract.

## Runtime guardrails

Default limits are defined in `Limits` and can be lowered by trusted config:

- `maxInputBytes`
- `maxDatasets`
- `maxColumnsPerDataset`
- `maxRowsPerDataset`
- `maxCellsPerDataset`
- `maxMetrics`
- `maxViews`
- `maxAlerts`
- `maxMarkdownSections`
- `maxStringChars`
- `maxMarkdownSectionChars`
- `maxTotalMarkdownChars`
- `warnOutputHtmlBytes`

Validation fails closed for oversized or structurally invalid payloads. Render
commands warn when generated HTML exceeds `warnOutputHtmlBytes`; with
`--warnings-as-errors`, this blocks the command.

Trusted config may also set `themeTokens` for renderer colors. Theme token values
are validated as safe CSS color literals before render output is written; raw CSS
remains outside the payload contract.

## Release gates

CI blocks on:

1. Vue typecheck/build and committed generated assets.
2. `cargo audit` dependency advisory scan.
3. `cargo fmt --all -- --check`.
4. `cargo clippy --workspace --all-targets -- -D warnings`.
5. `cargo test --workspace`.
6. Release binary smoke verification via `scripts/verify-release.sh`.

## Non-goals

The renderer is not a sandbox for arbitrary UI code and does not execute payload
scripts. Hosts embedding generated HTML should still serve it with appropriate
CSP and origin isolation for their environment.

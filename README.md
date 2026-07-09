# agent-ui-render

Zero-install Agent UI renderer CLI.

`agent-ui-render` converts governed, token-efficient compact Agent UI payloads
into browser-openable previews. The runtime is a small Rust CLI; the visual
renderer is maintained as Vue components and bundled into the binary at release
time.

## Goals

- Users do **not** install Node, Bun, npm, Vue, or `node_modules`.
- The model-authored boundary is compact JSON with `"version": 1`.
- LLM output stays token-efficient; Rust normalizes it into a clean domain model.
- The CLI validates, normalizes, plans, and renders payloads.
- UI maintainers develop renderer UI with Vue components under `renderer-vue/`.
- Release artifacts are single native binaries plus installer scripts.

## Quick start from source

```bash
# Build the embedded Vue client renderer used by Rust include_str! assets.
cd renderer-vue
bun install
bun run build

# Build and run the CLI.
cd ..
cargo run -- validate examples/revenue-overview.input.json
cargo run -- render html examples/revenue-overview.input.json /tmp/revenue.html
cargo run -- render static-html examples/revenue-overview.input.json /tmp/revenue.static.html
```

Open `/tmp/revenue.html` in a browser.

## Compact input shape

```json
{
  "version": 1,
  "t": "Revenue Overview",
  "d": [["sales", [["month", "s"], ["revenue", "cur", "USD"]], [["Jan", 120000]]]],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
```

The compact wire format uses dataset indexes and short view/alert codes to keep
LLM output small. Compact code mappings live in `wire::compact`; `normalize`
expands them into the clean `domain::Report` model with
`schema: "ui.input.normalized"` and `version: 1` before planning or rendering.

## CLI surface

```bash
agent-ui-render validate <input.json>
agent-ui-render normalize <input.json> [output.json]
agent-ui-render plan <input.json> [output.json]
agent-ui-render render html <input.json> <output.html>
agent-ui-render render static-html <input.json> <output.html>
agent-ui-render render vue <input.json> <output.vue>
agent-ui-render schema print <compact|normalized|spec|config>
agent-ui-render completion <shell>
```

Global flags:

```bash
-o, --output <human|json>
--warnings-as-errors
--quiet
--pretty
--config <path>
```

## Configurable limits and theme colors

Use an explicit config file to override runtime guardrails and trusted host color
tokens:

```bash
agent-ui-render --config agent-ui-render.config.json render html input.json report.html
```

```json
{
  "limits": {
    "maxInputBytes": 5242880,
    "maxRowsPerDataset": 2000,
    "maxCellsPerDataset": 100000,
    "warnOutputHtmlBytes": 5242880
  },
  "themeTokens": {
    "page": "#0b1220",
    "bg": "#111827",
    "surface": "#1f2937",
    "text": "#f9fafb",
    "primary": "#8b5cf6",
    "series1": "#8b5cf6",
    "series2": "#06b6d4"
  }
}
```

Limits and theme tokens are host/runtime policy and are never read from the
untrusted payload. Theme tokens map to `--agent-*` CSS custom properties and are
validated as safe CSS color literals before render output is written.

## Architecture

```text
renderer-vue/src/**/*.vue
        |
        | Vite build, development/release only
        v
generated/renderer.js + generated/renderer.css
        |
        | Rust include_str!
        v
agent-ui-render binary
        |
        +--> validate compact wire input
        +--> normalize wire::compact -> domain::Report
        +--> plan spec::plan_ui_spec
        +--> render html        # embedded Vue client renderer
        +--> render static-html # Rust no-JS fallback
        +--> render vue         # SFC wrapper + handoff source bundle
```

## Verification

```bash
cd renderer-vue && bun run typecheck && bun run build
cd ..
cargo audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/verify-release.sh
```

## Contract

See:

- `skills/agent-ui-render/references/ui-input.md`
- `skills/agent-ui-render/references/dataset.md`
- `docs/architecture.md`
- `docs/compatibility.md`
- `docs/security-model.md`
- `docs/renderer-development.md`
- `docs/release.md`
- `schemas/v1/*.schema.json`
- `schemas/config.schema.json`

# Architecture

`agent-ui-render` separates runtime distribution from UI authoring.

```text
Development time
----------------
Vue SFC renderer
  AgentUiRenderer.vue
  components/*.vue
  agent-ui.css
        |
        v
Vite bundle
  generated/renderer.js
  generated/renderer.css
        |
        v
Rust include_str!

User runtime
------------
input.json (wire::compact)
   |
   v
agent-ui-render
   |
   +--> validate compact wire input
   +--> normalize to domain::Report (schema=ui.input.normalized, version=1)
   +--> plan spec::plan_ui_spec (schema=ui.spec, version=1) when requested
   +--> render self-contained HTML
```

## Crates

```text
crates/agent-ui-render-cli
  clap command surface, IO, exit codes, shell completions

crates/agent-ui-render-core
  wire/compact.rs      compact LLM input and short-code mappings
  domain/report.rs     Report, Dataset, Metric, ViewIntent, Alert, constants
  normalize/           wire -> domain entrypoint
  spec/                UI spec planner
  validate/            Rust validator and unsafe-content checks
  chart/, markdown/    shared rendering logic with Vue parity tests
  render/              static renderer, HTML shell, embedded assets
```

## Renderer modes

| Mode | Command | Runtime dependency | Notes |
| --- | --- | --- | --- |
| Vue client HTML | `render html` | Browser JS | Rich preview |
| Static HTML | `render static-html` | None | No-JS fallback |
| Vue handoff | `render vue` | Vue app build | Source handoff |

The default HTML is not SSR. It embeds normalized payload JSON plus the bundled
Vue renderer. For no-JS artifacts, use `render static-html`.

## Source of truth

In this architecture, Rust is the runtime source of truth for:

- validation
- normalization
- planning
- static rendering

Vue remains the source of truth for visual component maintenance in the client
preview and handoff bundle.

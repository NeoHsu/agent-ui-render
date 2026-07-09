# Renderer Development

The user-facing CLI does not require Node or Bun. Vue tooling is only used while
developing and releasing the embedded renderer.

## Build renderer assets

```bash
cd renderer-vue
bun install
bun run typecheck
bun run build
```

The build writes:

```text
generated/renderer.js
generated/renderer.css
```

Rust embeds these files through `include_str!`, so they must exist before
`cargo build`.

## Development rules

- Keep renderer payload data as normalized reports (`schema: "ui.input.normalized"`,
  `version: 1`); do not add arbitrary component names, event handlers, or styles
  to the payload contract.
- Use `agent-ui.css` tokens for themes.
- Keep chart decision semantics aligned with Rust `chart_kind_for_view`.
- Run Rust and Vue checks before release.

## Handoff bundle

`agent-ui-render render vue input.json Report.vue` writes:

```text
Report.vue
agent-ui-renderer/
  AgentUiRenderer.vue
  components/*.vue
  agent-ui.css
  chart-model.ts
  chart-selection.ts
  format.ts
  markdown.ts
  types.ts
```

These files are embedded into the binary from `renderer-vue/src/`.

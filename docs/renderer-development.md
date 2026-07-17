# Renderer Development

The user-facing CLI does not require Node or Bun. Vue tooling is only used while
developing and releasing the embedded renderer.

## Build renderer assets

```bash
make setup
make generate
```

Equivalent direct commands:

```bash
cd renderer-vue
bun install
bun run typecheck
bun run test
bun run build
```

The build writes:

```text
generated/renderer.js
generated/renderer.css
```

Rust embeds these files through `include_str!`, so they must exist before
`cargo build` or release verification.

## Renderer build flow

```text
+-----------------------------+
| renderer-vue/src            |
| Vue components, CSS, TS     |
+--------------+--------------+
               |
               | vue-tsc + Vitest validate
               | contracts and behavior
               v
+-----------------------------+
| Verified renderer source    |
+--------------+--------------+
               |
               | Vite bundles client assets
               v
+-----------------------------+
| generated/renderer.js       |
| generated/renderer.css      |
+--------------+--------------+
               |
               | Rust include_str!
               v
+-----------------------------+
| agent-ui-render render html |
+-----------------------------+
```

## Development rules

- Keep renderer payload data as normalized reports (`schema: "ui.input.normalized"`,
  `version: 1` or `version: 2`); do not add arbitrary component names, event
  handlers, styles, or raw Vega-Lite specifications to the payload contract.
- Use `agent-ui.css` tokens for themes; public config `themeTokens` may override
  color tokens, so new renderer colors should be represented as `--agent-*`
  custom properties instead of hard-coded values.
- Keep v1 chart decision semantics aligned with Rust `chart_kind_for_view`.
- Version 2 Vega-Lite specs come from the trusted Rust planner. The Vue renderer
  only attaches normalized dataset rows and owns Vega View lifecycle cleanup.
- Vega's loader must continue rejecting all external network/file resources.
- Keep the generated HTML CSP hash-only for inline scripts/styles. Vega currently
  requires `'unsafe-eval'` for trusted planner-generated expressions; do not add
  `'unsafe-inline'` or external connection sources.
- Keep focused unit and component coverage under `renderer-vue/tests`; browser-only
  interactions and serious/critical axe accessibility checks remain in
  `scripts/interaction-smoke.ts`.
- Keep Rust static Markdown and Vue Markdown security behavior aligned through
  `fixtures/markdown-security.json`; add adversarial cases there before changing
  link or escaping policy.
- Commit `renderer-vue/src` changes and generated asset changes together.
- Run Rust and Vue checks before release.

## Handoff bundle

`agent-ui-render render vue input.json Report.vue` writes:

```text
+-----------------------+----------------------------------+
| Output path           | Purpose                          |
+-----------------------+----------------------------------+
| Report.vue            | wrapper with normalized payload  |
| agent-ui-renderer/    | adjacent renderer source bundle  |
|   AgentUiRenderer.vue | root renderer component          |
|   components/**       | renderer child components        |
|   agent-ui.css        | renderer styles and tokens       |
|   *.ts                | chart, format, markdown, types   |
+-----------------------+----------------------------------+
```

Handoff flow:

```text
+----------------------------+
| Compact input JSON         |
+-------------+--------------+
              |
              v
+----------------------------+
| Validate and normalize     |
+-------------+--------------+
              |
              v
+----------------------------+
| render vue                 |
+-------------+--------------+
              |
      +-------+-------------------+
      |                           |
      v                           v
+------------------+   +------------------------+
| Report.vue       |   | agent-ui-renderer/     |
| payload wrapper  |   | copied source bundle   |
+--------+---------+   +-----------+------------+
         |                         |
         +-----------+-------------+
                     |
                     v
+----------------------------+
| Downstream Vue app imports |
| wrapper and source bundle  |
+----------------------------+
```

These handoff files are embedded into the binary from `renderer-vue/src/`; update
that source directory before rebuilding generated assets. The CLI stages and
syncs the complete bundle before replacing a managed destination. It refuses to
delete an unknown `agent-ui-renderer/` directory unless the caller passes
`--force`.

# Development

This guide is for maintainers and coding agents modifying this repository.

## Local setup

```bash
mise install
make setup
make generate
make dev
```

Stable task entry points:

```text
make setup          install renderer dependencies
make generate       build generated/renderer.js and generated/renderer.css
make dev            build assets and run CLI help from source
make typecheck      run Vue and Rust type checks
make lint           run typecheck, rustfmt check, and clippy
make test           build assets and run Rust tests
make audit          run Cargo and Bun dependency audits
make docs-check     check docs/cli-reference.md against CLI --help output
make examples-check validate markdown payload examples against the CLI
make check          run release-quality local checks
make verify-release run release binary smoke verification
make visual-smoke   build visual smoke artifacts
make interaction-smoke exercise tooltip and selection UX in headless Chrome
```

Capture the tooltip, click, legend, brush, and zoom states during that smoke
run by setting an output directory:

```bash
INTERACTION_SCREENSHOT_DIR=target/visual-smoke/interactions make interaction-smoke
```

## Documentation diagram style

Use fenced `text` blocks with ASCII-only diagrams for architecture and flow
sections. Prefer labeled boxes, aligned columns, and clear branch paths over
loose arrow lists. Do not use Mermaid, image-only diagrams, or Unicode arrows in
project workflow docs; agents should be able to read the diagram in plain text.

```text
+---------+
| Source  |
+----+----+
     |
     v
+----------+
| Step one |
+----+-----+
     |
     +----------+
     |          |
     v          v
+----------+  +----------+
| Branch A |  | Branch B |
+----------+  +----------+
```

## Repository architecture

```text
+-------------------------------+--------------------------------------+
| Repository area               | Responsibility                       |
+-------------------------------+--------------------------------------+
| crates/agent-ui-render-cli    | CLI parsing, IO, output, exit codes  |
| crates/agent-ui-render-core   | Wire, domain, validation, rendering  |
| renderer-vue/src              | Vue renderer source at build time    |
| generated                     | Bundled JS/CSS embedded by Rust      |
| schemas                       | JSON Schema mirrors                  |
| examples                      | Compact input smoke data             |
| docs                          | User, maintainer, release docs       |
| skills/agent-ui-render        | Agent-facing payload guide           |
| scripts                       | Release and visual smoke checks      |
| Makefile                      | Stable local task entry points       |
+-------------------------------+--------------------------------------+
```

## Build and runtime flow

```text
DEVELOPMENT / RELEASE

+-------------------------+      bun + Vite      +------------------------+
| renderer-vue/src        | -------------------> | generated/renderer.*  |
| Vue, CSS, TS sources    |                      | embedded JS/CSS       |
+-------------------------+                      +-----------+------------+
                                                             |
                                                             | include_str!
                                                             v
                                                 +------------------------+
                                                 | agent-ui-render binary |
                                                 +-----------+------------+
                                                             |
                                                             v
USER RUNTIME                                      +------------------------+
                                                 | compact input JSON     |
                                                 +-----------+------------+
                                                             |
                                                             v
                                                 +------------------------+
                                                 | validate + normalize   |
                                                 +-----------+------------+
                                                             |
             +-----------------------------------+----------+-------------+
             |                                              |
             v                                              v
+----------------------------+                 +--------------------------+
| plan ui.spec JSON          |                 | render html/static/vue   |
+----------------------------+                 +--------------------------+
```

## Change map

```text
+----------------------+-----------------------------------------------+
| Change target        | Primary files to inspect                     |
+----------------------+-----------------------------------------------+
| CLI behavior         | crates/agent-ui-render-cli, e2e tests, docs  |
| Payload contract     | wire, domain, validate, normalize, schemas   |
| Planning/charts      | spec, chart, wire/v2, Vega builder, Vue charts|
| Static HTML          | core render module and core tests            |
| Vue renderer UI      | renderer-vue/src plus generated assets       |
+----------------------+-----------------------------------------------+
```

## Renderer change workflow

```text
+-----------------------------+
| Edit renderer-vue/src       |
+-------------+---------------+
              |
              v
+-----------------------------+
| make generate               |
| rebuild generated assets    |
+-------------+---------------+
              |
              v
+-----------------------------+
| make lint                   |
| Vue typecheck + Rust lint   |
+-------------+---------------+
              |
              v
+-----------------------------+
| Render an example HTML      |
+-------------+---------------+
              |
              v
+-----------------------------+
| Commit source and generated |
| asset changes together      |
+-----------------------------+
```

## Contract change workflow

```text
+------------------------------+
| Propose contract change      |
+--------------+---------------+
               |
               v
+------------------------------+
| Check compatibility policy   |
| version impact and safety    |
+--------------+---------------+
               |
               v
+------------------------------+
| Update Rust source of truth  |
| wire/domain modules          |
| validate and normalize       |
+--------------+---------------+
               |
               v
+------------------------------+
| Update schemas and examples  |
+--------------+---------------+
               |
               v
+------------------------------+
| Update skill refs and docs   |
+--------------+---------------+
               |
               v
+------------------------------+
| Update renderer/tests when   |
| visual behavior changes      |
+--------------+---------------+
               |
               v
+------------------------------+
| make check                   |
+------------------------------+
```

For compact v2 chart work, also update `schemas/v2`,
`skills/agent-ui-render/references/charts-v2.md`, the trusted Vega-Lite builder,
and browser no-network smoke coverage.

A contract change is not complete until all of these agree:

- Rust domain structs/constants
- Rust compact wire mappings
- Rust validators
- Rust normalizer
- planner/static renderer when applicable
- Vue renderer types/components when applicable
- `schemas/` and config schema when applicable
- `examples/`
- skill reference docs (`skills/agent-ui-render/`)
- tests

## Verification before finishing

Use the smallest useful check while iterating, then broaden before declaring the
work complete.

```text
+---------------------------+------------------------------------------+
| Change scope              | Verification                             |
+---------------------------+------------------------------------------+
| Docs only                 | Inspect Markdown and diagrams            |
| Payload/example           | validate plus one render command         |
| Rust only                 | cargo test --workspace                   |
| Vue renderer              | make generate plus make lint             |
| Contract/security/release | make check                               |
+---------------------------+------------------------------------------+
```

`make check` intentionally mirrors release-quality gates and may take longer
than narrow checks.

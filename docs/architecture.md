# Architecture

`agent-ui-render` separates agent-authored data, runtime validation, and visual
renderer authoring. Agents produce compact JSON. Rust turns that JSON into a
trusted runtime model. Vue is used only to maintain the rich browser renderer and
is bundled into the binary before release.

## System overview

```text
+----------------------------+
| Agent or host data source  |
+-------------+--------------+
              |
              v
+----------------------------+
| Compact JSON payload       |
| version: 1                 |
+-------------+--------------+
              |
              v
+----------------------------+
| agent-ui-render binary     |
| Rust runtime               |
+-------------+--------------+
              |
              v
+----------------------------+
| Validate compact input     |
+-------------+--------------+
              |
              v
+----------------------------+
| Normalize to domain model  |
| domain::Report             |
+-------------+--------------+
              |
     +--------+--------+----------------+
     |                 |                |
     v                 v                v
+------------+  +---------------+  +----------------+
| Plan       |  | Render HTML   |  | Render Vue     |
| ui.spec    |  | or static     |  | handoff bundle |
+------------+  +---------------+  +----------------+
```

## Build-time asset flow

```text
DEVELOPMENT / RELEASE TIME

+----------------------------+      bun + Vite      +----------------------+
| renderer-vue/src           | -------------------> | generated/renderer.* |
| - AgentUiRenderer.vue      |                      | - renderer.js        |
| - components/*.vue         |                      | - renderer.css       |
| - agent-ui.css             |                      +----------+-----------+
| - TypeScript helpers       |                                 |
+----------------------------+                                 |
                                                               | include_str!
                                                               v
                                                   +------------------------+
                                                   | release binary         |
                                                   | agent-ui-render        |
                                                   +-----------+------------+
                                                               |
                                                               v
INSTALLED USER RUNTIME                              +------------------------+
                                                    | native binary only     |
                                                    | no Node/Bun/npm/Vue    |
                                                    +------------------------+
```

## Runtime data flow

```text
+------------------------------+
| input.json                   |
| compact payload              |
+---------------+--------------+
                |
                v
+------------------------------+
| wire::compact                |
| parse tuples and short codes |
+---------------+--------------+
                |
                v
+------------------------------+
| validate                     |
| reject bad refs, unsafe      |
| content, and limit breaches  |
+---------------+--------------+
                |
                v
+------------------------------+
| normalize                    |
| expand labels, dictionary    |
| values, and references       |
+---------------+--------------+
                |
                v
+------------------------------+
| domain::Report               |
| schema=ui.input.normalized   |
+---------------+--------------+
                |
      +---------+---------+-------------------+
      |                   |                   |
      v                   v                   v
+-------------+    +--------------+    +----------------+
| ui.spec     |    | HTML outputs |    | Vue handoff    |
| plan JSON   |    | client/no-JS |    | source bundle  |
+-------------+    +--------------+    +----------------+
```

## Repository map

```text
+-------------------------------+--------------------------------------+
| Path                          | Role                                 |
+-------------------------------+--------------------------------------+
| crates/agent-ui-render-cli    | CLI surface, IO, output, exits       |
| crates/agent-ui-render-core   | Wire, domain, validation, rendering  |
| renderer-vue/src              | Vue/CSS/TS renderer source           |
| generated                     | Embedded renderer JS and CSS         |
| schemas                       | JSON Schema mirrors                  |
| examples                      | Compact payload smoke inputs         |
| skills/agent-ui-render        | Agent-facing authoring contract      |
| docs                          | User, maintainer, release guidance   |
+-------------------------------+--------------------------------------+
```

## Renderer modes

| Mode | Command | Runtime dependency | Notes |
| --- | --- | --- | --- |
| Vue client HTML | `render html` | Browser JS | Rich preview |
| Static HTML | `render static-html` | None | Rust no-JS fallback |
| Vue handoff | `render vue` | Vue app build | Source handoff |

The default HTML is not SSR. It embeds normalized payload JSON plus the bundled
Vue renderer. For no-JS artifacts, use `render static-html`.

## Source of truth

```text
+--------------------------+-----------------------------------------+
| Concern                  | Source of truth                         |
+--------------------------+-----------------------------------------+
| Public payload contract  | Rust wire parser and validator          |
| Runtime semantics        | Rust domain::Report and normalize/spec  |
| Static rendering         | Rust render module                      |
| Rich browser visuals     | renderer-vue/src, then generated assets |
| Schema integration       | schemas/ mirrors Rust behavior          |
| Host policy              | trusted config, never payload fields    |
+--------------------------+-----------------------------------------+
```

Rust is the runtime source of truth for validation, normalization, planning, and
static rendering. Vue remains the source of truth for visual component
maintenance in the client preview and handoff bundle.

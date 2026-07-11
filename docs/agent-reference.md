# Agent Reference

This file is a task router for coding agents that use or maintain
`agent-ui-render`. It is intentionally shorter than the full docs: read this
first, then follow the linked task-specific file.

## Agent roles

```text
                         +----------------+
                         | Incoming task  |
                         +-------+--------+
                                 |
                +----------------+----------------+
                |                                 |
                v                                 v
+-------------------------------+   +-------------------------------+
| Consumer Agent                |   | Maintainer Agent              |
| Create UI payload or preview  |   | Modify this repository        |
+---------------+---------------+   +---------------+---------------+
                |                                   |
                v                                   v
+-------------------------------+   +-------------------------------+
| Read docs/usage.md            |   | Read docs/development.md      |
| Read ui-input reference       |   | Read docs/architecture.md     |
+-------------------------------+   +-------------------------------+
```

## Task routing

```text
+-------------------------------+--------------------------------------+
| Task                          | Read first                           |
+-------------------------------+--------------------------------------+
| Validate or render payload    | usage, cli-reference, troubleshooting|
| Author compact payload        | usage, ui-input reference, dataset   |
| Change CLI behavior           | development, cli-reference, CLI crate|
| Change core runtime           | development, architecture, security  |
| Change Vue renderer           | renderer-development, renderer-vue   |
| Change payload contract       | compatibility, security, schemas     |
| Prepare or inspect release    | release, scripts/verify-release.sh   |
+-------------------------------+--------------------------------------+
```

## Required boundaries

- The model-authored public boundary is compact JSON: version 1 for semantic
  reports and version 2 for explicit governed chart families.
- Agents must not smuggle HTML, CSS, JavaScript, Vue, React, SVG, event handlers,
  arbitrary component names, or arbitrary action names into payloads.
- Rust validation, normalization, chart planning, and trusted Vega-Lite spec
  generation are the runtime source of truth.
- JSON Schemas mirror the Rust contract for integration checks.
- Vue is a development-time renderer source, not a runtime dependency for users.
- Runtime limits and theme tokens come from trusted host config, never from the
  untrusted payload.

## Standard agent workflow

```text
+-------------------------------+
| Understand the task           |
+---------------+---------------+
                |
                v
+-------------------------------+
| Read routed docs              |
+---------------+---------------+
                |
                v
+-------------------------------+
| Inspect smallest useful files |
+---------------+---------------+
                |
                v
+-------------------------------+
| Make focused edits            |
+---------------+---------------+
                |
                v
+-------------------------------+
| Run narrow verification       |
+---------------+---------------+
                |
                v
+-------------------------------+
| Run broader checks            |
+---------------+---------------+
                |
                v
+-------------------------------+
| Summarize files and results   |
+-------------------------------+
```

## Verification

Per-scope verification commands live in `docs/development.md` under
"Verification before finishing" (the home copy — do not duplicate it here).
Use `make help` to list stable local task entry points.

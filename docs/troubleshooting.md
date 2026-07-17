# Troubleshooting

Use this guide when validation, rendering, local development, or release checks
fail.

## Triage flow

```text
+------------------+
| Command failed   |
+--------+---------+
         |
         +-------------------+-------------------+-------------------+
         |                   |                   |                   |
         v                   v                   v                   v
+----------------+  +----------------+  +----------------+  +---------------+
| CLI usage      |  | Validation     |  | Render output  |  | Build or CI   |
| error          |  | error          |  | missing/broken |  | failure       |
+-------+--------+  +-------+--------+  +-------+--------+  +-------+-------+
        |                   |                   |                   |
        v                   v                   v                   v
+----------------+  +----------------+  +----------------+  +---------------+
| cli-reference  |  | fix JSON, refs |  | check assets   |  | make setup or |
| docs           |  | unsafe, limits |  | and validity   |  | make check    |
+----------------+  +----------------+  +----------------+  +---------------+
```

## Validation failures

### Invalid JSON

Symptoms:

- `validate` fails before reporting field-specific contract errors.
- JSON tooling cannot parse the file.

Actions:

```bash
python3 -m json.tool input.json >/dev/null
agent-ui-render validate input.json
```

### Unsafe HTML, CSS, JavaScript, or component content

Symptoms:

- Validation rejects strings containing raw tags, scripts, styles, event
  handlers, `javascript:` URLs, or component/action injection patterns.

Actions:

- Remove raw UI/code content from the payload.
- Use datasets for records and safe markdown text under `md` for prose.
- Use alerts under `a` to explain omissions or uncertainty.

```text
+-----------------------+----------------------------------+
| Unsafe payload shape  | Safe replacement                 |
+-----------------------+----------------------------------+
| raw table/chart HTML  | dataset under d + view tuple     |
| prose formatting HTML | safe markdown under md           |
| executable behavior   | remove; outside the contract     |
| unknown chart intent  | records view + explanatory alert |
+-----------------------+----------------------------------+
```

### Bad dataset or column indexes

Symptoms:

- A view references a dataset index or column index that does not exist.
- Chart validation fails because x/measure columns are incompatible.

Actions:

- Count datasets in top-level `d` from zero.
- Count columns inside the chosen dataset from zero.
- Use numeric-compatible measure columns for charts: `n`, `cur`, or `pct`.
- Fall back to `r` records view plus an alert when chart intent is uncertain.

### Limit failures or output-size warnings

Symptoms:

- Input is too large.
- Too many datasets, dictionaries, rows, cells, metrics, views, alerts, diagnostics, or markdown sections.
- Total rows or cells exceed the cross-dataset budget.
- Render warns about generated HTML size or rejects the hard output-size limit.

Actions:

- Reduce the payload to the rows and views needed for the report.
- Use external dataset refs only when the host UI can resolve them.
- Lower or raise trusted host limits through `--config`; never put limits inside
  the untrusted payload.
- Remove `--warnings-as-errors` while debugging warnings that are acceptable;
  restore it before producing release or CI artifacts.

## Render failures

### Generated renderer assets are missing

Symptoms:

- Rust build fails on embedded assets.
- HTML render cannot include `generated/renderer.js` or `generated/renderer.css`.

Actions:

```bash
make setup
make generate
cargo build --workspace
```

### Static HTML differs from client HTML

The static renderer is a no-JS fallback. It should preserve validated content and
safe rendering, but it is not expected to match every client-side visual detail.

Actions:

- Use `render html` for rich browser previews.
- Use `render static-html` for portable no-JS artifacts.
- Add tests when behavior, formatting, or safety should be identical.

### Vue handoff bundle missing files

Symptoms:

- `render vue` writes a wrapper but downstream app cannot import adjacent source
  files.

Actions:

```bash
agent-ui-render render vue input.json Report.vue
find agent-ui-renderer -maxdepth 2 -type f | sort
```

The wrapper and `agent-ui-renderer/` directory must move together. Generated
bundles contain `.agent-ui-render-managed`; if an existing directory is
unmanaged, move it aside or review it before deliberately passing `--force`.

## Development failures

### Bun, Rust, actionlint, or cargo-audit is missing

Actions:

```bash
mise install
make setup
cargo install cargo-audit --locked
```

### Generated asset drift in CI

Symptoms:

- CI fails on `git diff --exit-code generated/ renderer-vue/src/`.

Actions:

```bash
make generate
git diff -- generated/ renderer-vue/src/
```

Commit source and generated asset changes together.

### Clippy or rustfmt fails

Actions:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Fix warnings instead of suppressing them unless the suppression is documented and
necessary.

## Release check failures

`make check` runs the release-quality path:

```text
+-----------------------------+
| make check                  |
+--------------+--------------+
               |
               v
+-----------------------------+
| Generate embedded assets    |
+--------------+--------------+
               |
               v
+-----------------------------+
| Cargo + Bun audits          |
+--------------+--------------+
               |
               v
+-----------------------------+
| Typecheck, rustfmt, clippy   |
+--------------+--------------+
               |
               v
+-----------------------------+
| Rust 1.91 MSRV check        |
+--------------+--------------+
               |
               v
+-----------------------------+
| Vitest + Rust tests         |
+--------------+--------------+
               |
               v
+-----------------------------+
| Docs, examples, release,    |
| and browser smoke checks    |
+-----------------------------+
```

If a gate fails, fix the first failing gate before rerunning the full check.
Use `docs/release.md` for the release-specific publishing flow.

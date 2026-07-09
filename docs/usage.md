# Usage

This guide is for users and agents that want to create or render Agent UI
payloads. It assumes `agent-ui-render` is already installed or available through
`cargo run --` from this repository.

## End-to-end workflow

```text
+--------------------------+
| Facts, data, analysis    |
+------------+-------------+
             |
             v
+--------------------------+
| Author compact JSON      |
| version: 1               |
+------------+-------------+
             |
             v
+--------------------------+
| validate input.json      |
+------------+-------------+
             |
      +------+------+
      |             |
      v             v
+-----------+  +---------------------+
| invalid   |  | valid               |
| fix JSON  |  | continue to render  |
+-----+-----+  +----------+----------+
      |                   |
      +-------------------+
                          |
                          v
             +--------------------------+
             | render html              |
             | optional: static/plan    |
             +------------+-------------+
                          |
                          v
             +--------------------------+
             | Open or share artifact   |
             +--------------------------+
```

## Authoring rules for agents

The authoring contract's home is `skills/agent-ui-render/` (SKILL.md plus its
references). Read it before producing non-trivial payloads:

- `skills/agent-ui-render/references/ui-input.md` — full payload contract.
- `skills/agent-ui-render/references/dataset.md` — dataset and column rules.

Core rules, in brief: output compact JSON only; put shared tabular data under
top-level `d` and reference it by indexes from views; use `md` and alerts for
narrative and caveats; never write HTML, CSS, JavaScript, or component/action
names into payload strings.

## Minimal compact payload

```json
{
  "version": 1,
  "t": "Revenue Overview",
  "s": "Two months of revenue data.",
  "d": [
    [
      "sales",
      [["month", "s"], ["revenue", "cur", "USD"]],
      [["Jan", 120000], ["Feb", 135000]]
    ]
  ],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
```

## Validate

```bash
agent-ui-render validate input.json
```

Use JSON output when a calling agent or host tool needs machine-readable results:

```bash
agent-ui-render -o json validate input.json
```

Treat warnings as blocking when producing release or CI artifacts:

```bash
agent-ui-render --warnings-as-errors validate input.json
```

## Render modes

```text
                         +----------------------+
                         | input.json           |
                         | compact payload      |
                         +----------+-----------+
                                    |
          +-------------------------+-------------------------+
          |                         |                         |
          v                         v                         v
+-------------------+     +--------------------+     +--------------------+
| render html       |     | render static-html |     | render vue         |
| report.html       |     | report.static.html |     | Report.vue bundle  |
| rich preview      |     | no-JS fallback     |     | app handoff        |
+-------------------+     +--------------------+     +--------------------+
```

Commands:

```bash
agent-ui-render render html input.json report.html
agent-ui-render render static-html input.json report.static.html
agent-ui-render render vue input.json Report.vue
```

## Debug and integration outputs

Normalize compact input into the readable runtime model:

```bash
agent-ui-render --pretty normalize input.json normalized.json
```

Plan the canonical UI spec:

```bash
agent-ui-render --pretty plan input.json spec.json
```

Print JSON Schemas:

```bash
agent-ui-render schema print compact
agent-ui-render schema print normalized
agent-ui-render schema print spec
agent-ui-render schema print config
```

## Trusted config

Payloads cannot define runtime limits or CSS. Hosts may provide trusted config:

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
    "primary": "#8b5cf6"
  }
}
```

## Common recovery path

```text
+------------------+
| Command failed   |
+--------+---------+
         |
         +-----------------------+-----------------------+
         |                       |                       |
         v                       v                       v
+------------------+   +------------------+   +----------------------+
| Payload rejected |   | Render failed    |   | Source build failed  |
| validation docs  |   | render failures  |   | development docs     |
+------------------+   +------------------+   +----------------------+
```

# CLI Reference

## `validate`

```bash
agent-ui-render validate input.json
```

Validates compact version 1 input.

## `normalize`

```bash
agent-ui-render normalize input.json normalized.json
agent-ui-render --pretty normalize input.json
```

Outputs a normalized report with `schema: "ui.input.normalized"` and
`version: 1`.

## `plan`

```bash
agent-ui-render plan input.json spec.json
```

Outputs a canonical UI spec with `schema: "ui.spec"` and `version: 1`.

## `render html`

```bash
agent-ui-render render html input.json report.html
```

Writes a self-contained HTML file with embedded CSS, normalized payload, and the
Vue client renderer.

## `render static-html`

```bash
agent-ui-render render static-html input.json report.static.html
```

Writes a no-JS static HTML fallback.

## `render vue`

```bash
agent-ui-render render vue input.json Report.vue
```

Writes a Vue SFC wrapper and adjacent `agent-ui-renderer/` handoff source
bundle.

## `schema print`

```bash
agent-ui-render schema print compact
agent-ui-render schema print normalized
agent-ui-render schema print spec
agent-ui-render schema print config
```

## Config

```bash
agent-ui-render --config agent-ui-render.config.json validate input.json
```

Config files override built-in runtime limits. Example:

```json
{
  "limits": {
    "maxInputBytes": 5242880,
    "maxRowsPerDataset": 2000,
    "maxCellsPerDataset": 100000,
    "warnOutputHtmlBytes": 5242880
  }
}
```

Available limit keys:

```text
maxInputBytes
maxDatasets
maxColumnsPerDataset
maxRowsPerDataset
maxCellsPerDataset
maxMetrics
maxViews
maxAlerts
maxMarkdownSections
maxStringChars
maxMarkdownSectionChars
maxTotalMarkdownChars
warnOutputHtmlBytes
```

## Exit codes

| Code | Meaning                     |
| ---: | --------------------------- |
|    0 | Success                     |
|    1 | Validation or runtime error |
|    2 | CLI usage error             |
|    3 | Warning treated as error    |
|    4 | IO error                    |

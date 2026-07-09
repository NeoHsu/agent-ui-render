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

Config files override built-in runtime limits and may provide trusted host theme
color tokens. Limits and theme tokens are host/runtime policy and are never read
from the untrusted input payload.

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
    "muted": "#cbd5e1",
    "primary": "#8b5cf6",
    "accent": "#f59e0b",
    "series1": "#8b5cf6",
    "series2": "#06b6d4"
  }
}
```

`themeTokens` are emitted after the bundled renderer CSS for `render html`,
`render static-html`, and `render vue`. Values must be safe CSS color literals:
hex colors, common CSS color functions like `rgb(...)`/`oklch(...)`,
`transparent`, or `currentColor`.

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

Available theme token keys:

```text
page
bg
surface
surfaceMuted
surfaceStrong
border
borderSoft
text
muted
subtle
primary
accent
info
success
error
codeBg
codeFg
codeBorder
preBg
preFg
preBorder
chartBg
chartBorder
chartAxis
series1
series2
series3
series4
series5
series6
criticalBg
criticalSoft
criticalBorder
criticalFg
errorBg
errorSoft
errorBorder
errorFg
warningBg
warningBorder
warningFg
successBg
successBorder
successFg
infoBg
infoBorder
infoFg
```

## Exit codes

| Code | Meaning                     |
| ---: | --------------------------- |
|    0 | Success                     |
|    1 | Validation or runtime error |
|    2 | CLI usage error             |
|    3 | Warning treated as error    |
|    4 | IO error                    |

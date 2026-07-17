# CLI Reference

`agent-ui-render --help` (and each subcommand's `--help`) is the source of
truth; this file is a navigational overview. `scripts/check-cli-docs.sh`, run
by `make check` and CI, fails when a command or global flag documented by
`--help` goes missing from this file.

## Command lifecycle

```text
+------------------------------------+
| agent-ui-render [flags] <command>  |
+-----------------+------------------+
                  |
+--------------------+-----------------------------------------+
| Command            | Output or behavior                      |
+--------------------+-----------------------------------------+
| validate           | compact input validation                |
| normalize          | normalized report JSON                  |
| plan               | canonical ui.spec JSON                  |
| render html        | self-contained browser HTML             |
| render static-html | no-JS HTML fallback                     |
| render vue         | Vue wrapper plus handoff source bundle  |
| schema print       | JSON Schema document                    |
| completion         | shell completion script                 |
+--------------------+-----------------------------------------+
```

## Global flags

```bash
-o, --output <human|json>
--warnings-as-errors
--quiet
--pretty
--config <path>
```

Use `--config` for trusted host runtime policy such as limits and theme tokens.
Payloads cannot set those values.

## `validate`

```bash
agent-ui-render validate input.json
```

Validates compact version 1 semantic input or compact version 2 governed chart input.

Machine-readable output:

```bash
agent-ui-render -o json validate input.json
```

Block on warnings:

```bash
agent-ui-render --warnings-as-errors validate input.json
```

## `normalize`

```bash
agent-ui-render normalize input.json normalized.json
agent-ui-render --pretty normalize input.json
```

Outputs a normalized report with `schema: "ui.input.normalized"` and the
same version as the compact input.

## `plan`

```bash
agent-ui-render plan input.json spec.json
```

Outputs a canonical UI spec with `schema: "ui.spec"` and the same version as
the compact input.

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
agent-ui-render render vue input.json Report.vue --force
```

Writes a Vue SFC wrapper and adjacent `agent-ui-renderer/` handoff source
bundle. Existing managed bundles are replaced transactionally. An unmanaged
`agent-ui-renderer` path is preserved unless `--force` is explicit.

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

## `schema print`

```bash
agent-ui-render schema print compact
agent-ui-render schema print compact-v2
agent-ui-render schema print normalized
agent-ui-render schema print normalized-v2
agent-ui-render schema print spec
agent-ui-render schema print spec-v2
agent-ui-render schema print config
```

## `completion`

```bash
agent-ui-render completion bash
agent-ui-render completion zsh
agent-ui-render completion fish
agent-ui-render completion powershell
agent-ui-render completion elvish
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
  "documentLanguage": "en",
  "limits": {
    "maxInputBytes": 5242880,
    "maxRowsPerDataset": 2000,
    "maxCellsPerDataset": 100000,
    "maxTotalRows": 20000,
    "maxTotalCells": 250000,
    "maxFindings": 200,
    "warnOutputHtmlBytes": 5242880,
    "maxOutputHtmlBytes": 10485760
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

`documentLanguage` must be a safe BCP-47 language tag and defaults to `en`.
It controls the `lang` attribute on generated rich and static HTML documents.

`themeTokens` are emitted after the bundled renderer CSS for `render html`,
`render static-html`, and `render vue`. Values must be safe CSS color literals:
hex colors, common CSS color functions like `rgb(...)`/`oklch(...)`,
`transparent`, or `currentColor`.

Available limit keys:

```text
maxInputBytes
maxDatasets
maxDictionaries
maxDictionaryEntries
maxColumnsPerDataset
maxRowsPerDataset
maxCellsPerDataset
maxTotalRows
maxTotalCells
maxMetrics
maxViews
maxAlerts
maxMarkdownSections
maxStringChars
maxMarkdownSectionChars
maxTotalMarkdownChars
maxFindings
warnOutputHtmlBytes
maxOutputHtmlBytes
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

| Code | Meaning |
| ---: | --- |
| 0 | Success |
| 1 | Validation or runtime error |
| 2 | CLI usage error |
| 3 | Warning treated as error |
| 4 | IO error |

A closed stdout consumer (for example, `agent-ui-render ... | head`) is treated
as a successful early stop rather than an IO failure. Human-readable diagnostics
escape terminal control and bidirectional override characters; JSON mode retains
structured machine-readable output.

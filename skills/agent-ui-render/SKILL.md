---
name: agent-ui-render
description: >-
  Create governed renderable agent UI payloads and previews from data or
  analysis. Use whenever the user asks for a dashboard, chart, table, metric
  cards, visual/data report, structured UI payload, Vue component preview, or
  static HTML preview, or says "render this analysis", "show this as UI",
  "dashboard", "chart", "table", "metric card", "structured UI", or in Chinese
  「儀表板」「圖表」「表格」「報表」「指標卡」「資料視覺化」「渲染成 UI」
  「視覺化報告」. When the `agent-ui-render` CLI or this repository is
  available, this skill supersedes hand-authored chart/dashboard code: the
  deliverable is a validated compact JSON payload rendered by the CLI, never
  hand-written UI. The model authors compact JSON with `version: 1`; the
  bundled zero-install `agent-ui-render` CLI validates, normalizes, and renders
  previews. Do not use this skill for arbitrary frontend implementation,
  CSS/layout work, or hand-written renderer code.
compatibility: Requires the `agent-ui-render` binary (on PATH, or built at `target/release/` in this repo). No Node/Bun/npm is required at runtime.
---

# Agent UI Render

Generate browser-openable Agent UI HTML from data or analysis without asking the
model to hand-write UI. The model-authored boundary is compact JSON with
`version: 1`; the CLI validates, normalizes, optionally plans, then renders HTML.

```text
LLM compact input -> agent-ui-render validate -> normalize -> render HTML
```

## Precondition gate

Before authoring anything, locate the CLI and show the check's output:

```bash
command -v agent-ui-render \
  || ls "$(git rev-parse --show-toplevel)/target/release/agent-ui-render"
```

- Found on PATH: use `agent-ui-render` as written below.
- Only the repo binary exists: substitute the repo binary path printed above
  (or `cargo run --release -p agent-ui-render --`).
- Neither exists: still author compact JSON, label the deliverable
  `[UNVALIDATED]`, tell the user to build it with
  `cargo build --release -p agent-ui-render`, and never fall back to
  hand-written HTML/Vue/React output.

## Detailed contract (read before authoring)

The tables in this file are a quick reference only. Full semantics live in:

- `references/ui-input.md` — the complete payload contract. Read it before
  writing any payload that uses `m`, `md`, `dict`, themes, or more than one
  view.
- `references/dataset.md` — dataset, column, and row rules, external refs,
  dictionary encoding, and view/column compatibility. Read it before shaping
  any dataset.

## Output modes

- **Default preview mode:** create valid compact JSON, save it to a file, then
  run:

  ```bash
  agent-ui-render render html <input.json> <output.html>
  ```

  Return the HTML path.

- **Payload mode:** only when the user explicitly asks for JSON, a payload,
  schema output, or an API contract, return valid compact JSON with no Markdown
  fences. Payload mode still requires `agent-ui-render validate <input.json>`
  to exit 0 before returning; quote the validation output as evidence (or the
  `[UNVALIDATED]` label when the binary is unavailable).

- **Static fallback mode:** for a no-JS HTML artifact, run:

  ```bash
  agent-ui-render render static-html <input.json> <output.html>
  ```

- **Vue component/developer mode:** run:

  ```bash
  agent-ui-render render vue <input.json> <output.vue>
  ```

  This writes a wrapper and adjacent `agent-ui-renderer/` source bundle.

- **Debug/interop mode:** run:

  ```bash
  agent-ui-render normalize <input.json> <normalized.json>
  agent-ui-render plan <input.json> <spec.json>
  ```

- **Configured runtime limits:** use trusted host config, never payload fields:

  ```bash
  agent-ui-render --config agent-ui-render.config.json \
    render html <input.json> <output.html>
  ```

## Compact contract summary

Default payload shape:

```ts
type CompactReport = {
  version: 1;
  t?: string;       // title
  s?: string;       // summary
  theme?: "report-light" | "technical-dark" | "executive-clean";
  density?: "comfortable" | "compact";
  emphasis?: "strong" | "subtle";
  d?: DatasetTuple[];
  m?: MetricTuple[];
  v?: ViewTuple[];
  a?: AlertTuple[];
  md?: MarkdownTuple[];
  dict?: Record<string, string[]>;
};
```

Use shared datasets under `d`; views reference datasets and columns by indexes.
Do not output readable object-array rows, final chart specs, HTML, Vue, React,
class names, style, arbitrary components, or action handlers.

Beware: the letter `d` has three unrelated meanings depending on position —
top-level key `d` (datasets), view code `d` (distribution), and column type
code `d` (date). Always disambiguate by context.

Short view codes:

```text
o overview
r records/table
t trend/line
b comparison/bar
d distribution
p composition/pie-or-bar
s relationship/scatter
```

Short alert codes:

```text
i info
s success
w warning
e error
c critical
```

## Workflow

1. Extract facts and intent. Do not invent rows, metrics, units, or business
   meaning.
2. Put reusable tabular data under `d` with `[id, columns, rows]` by default.
3. Add metrics in `m`, concise summary in `s`, and caveats in `a`.
4. Select semantic view tuples such as `r`, `t`, `b`, `p`, `s`, or `d`.
5. Validate:

   ```bash
   agent-ui-render validate <input.json>
   ```

   Fix warnings, not just errors — `--warnings-as-errors` is the quality bar.

6. Render by default:

   ```bash
   agent-ui-render render html <input.json> <output.html>
   ```

## Safety boundaries

Never output or smuggle raw HTML, CSS, JavaScript, JSX/TSX, Vue templates,
`className`, inline `style`, event handlers, script/iframe markup,
`dangerouslySetInnerHTML`, arbitrary component names, arbitrary action handler
names, or `javascript:` URLs into payloads. Treat all strings as untrusted text.

## Self-check before final

- `agent-ui-render validate` passed with zero errors, or the deliverable is
  explicitly labeled `[UNVALIDATED]` because no binary was available.
- Default preview mode returns the generated `.html` path.
- Payload mode returns parseable JSON only, with validation evidence quoted.
- Data is shared through `d`; records are not duplicated across blocks.
- Markdown narrative uses `md`; no raw HTML or UI code appears in payload.

# Agent UI Input Contract Reference

The model-authored format is compact JSON with `version: 1`. It is designed to
minimize LLM output tokens while keeping the runtime safe and maintainable. It
contains shared data, field semantics, summary text, metrics, view intent, safe
markdown, and alerts. It is **not** HTML, Vue, React, SVG, CSS, or final UI
spec.

## Responsibility split

- **LLM:** output compact input data only.
- **Validation tool:** parse JSON, validate semantic references, reject unsafe
  UI/code content, and report recoverable warnings.
- **Normalization tool:** expand short keys and codes, derive labels, resolve
  dictionary values, convert indexes to keys, and emit normalized reports.
- **Planner:** map normalized view intent to canonical UI spec blocks.
- **Renderer:** render only validated normalized data.

```text
+------------------------------+
| LLM-authored compact input   |
| version: 1                   |
+---------------+--------------+
                |
                v
+------------------------------+
| validate_report              |
| structure, refs, safety      |
+---------------+--------------+
                |
                v
+------------------------------+
| normalize_report             |
| expand codes into domain     |
+---------------+--------------+
                |
                v
+------------------------------+
| Normalized report            |
| schema=ui.input.normalized   |
+---------------+--------------+
                |
        +-------+-------+----------------+
        |               |                |
        v               v                v
+---------------+ +-------------+ +----------------+
| Render HTML   | | Plan ui.spec| | Product UI     |
| preview       | | JSON        | | handoff        |
+---------------+ +-------------+ +----------------+
```

## Compact top-level schema

```ts
type CompactReport = {
  version: 1;
  t?: string;                         // title
  s?: string;                         // summary
  theme?: "report-light" | "technical-dark" | "executive-clean";
  density?: "comfortable" | "compact";
  emphasis?: "strong" | "subtle";
  d?: DatasetTuple[];                 // shared datasets
  m?: MetricTuple[];                  // metric tuples
  v?: ViewTuple[];                    // semantic view intent tuples
  a?: AlertTuple[];                   // alerts
  md?: MarkdownTuple[];               // safe markdown narrative sections
  dict?: Record<string, string[]>;    // optional repeated-string dictionary
};
```

## Minimal recommended payload

```json
{
  "version": 1,
  "d": [
    [
      "sales",
      [["month", "s"], ["revenue", "cur", "USD"], ["growth", "pct"]],
      [["Jan", 120000, 0.05], ["Feb", 135000, 0.125], ["Mar", 150000, 0.111]]
    ]
  ],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
```

## Datasets

Full dataset rules live in `references/dataset.md`. In short:

- Put reusable tabular data under top-level `d`.
- Use `[id, columns, rows]` row-major datasets by default.
- Use `[id, "cols", columns, columnData]` only when column-major data is clearly
  more compact.
- Use `[id, "ref", reference]` only when the host UI can resolve external data.
- Views reference dataset indexes, not dataset ids, to avoid repeating strings.
- Keep rows primitive, rectangular, and source-faithful.

## Column tuples and type codes

Column tuple shapes and type codes are defined in `references/dataset.md`
(the home file — see its "Column tuples" section). Quick recall only:
`[key, type, unit?, label?]`; type codes `s n cur pct d dt b dict:<id>`.

Beware: the letter `d` has three unrelated meanings depending on position —
top-level key `d` (datasets), view code `d` (distribution), and column type
code `d` (date). Always disambiguate by context.

## Metrics

```ts
type MetricTuple =
  | [label: string, value: Primitive]
  | [label: string, value: Primitive, format: TypeCode]
  | [label: string, value: Primitive, format: TypeCode, unit: string];
```

Example:

```json
"m": [["Latest Revenue", 150000, "cur", "USD"], ["Growth", 0.111, "pct"]]
```

## Markdown narrative sections

Use `md` for report prose that benefits from markdown syntax while remaining
renderer-controlled.

```ts
type MarkdownTuple = [content: string] | [title: string, content: string];
```

Supported markdown subset: headings, paragraphs, bold, emphasis, inline code,
fenced code blocks, ordered/unordered lists, blockquotes, horizontal rules, and
safe links (`https:`, `http:`, `mailto:`, `/relative`, `#anchor`). Inline
semantic tokens `{critical: ...}`, `{error: ...}`, `{warning: ...}`,
`{success: ...}`, `{info: ...}`, and `{muted: ...}` render as fixed styled spans
without allowing raw HTML.

## View tuples

Views describe what the user should see. They are not component specs.

```ts
type ViewTuple =
  | ["o", data: number]
  | ["r", data: number]
  | ["r", data: number, columns: number[]]
  | ["t", data: number, x: number, measures: number[]]
  | ["b", data: number, dimension: number, measures: number[]]
  | ["d", data: number, dimension: number]
  | ["d", data: number, dimension: number, measures: number[]]
  | ["p", data: number, dimension: number, measures: number[]]
  | ["s", data: number, x: number, measures: number[]];
```

Code mapping:

| Code | Meaning              | Renderer mapping        |
| ---- | -------------------- | ----------------------- |
| `o`  | overview             | text/summary intent     |
| `r`  | records              | table                   |
| `t`  | trend                | line chart              |
| `b`  | comparison           | grouped bar chart       |
| `d`  | distribution         | bar chart               |
| `p`  | composition          | pie when safe, else bar |
| `s`  | relationship         | scatter chart           |

For `r`, omit `columns` to show every column, or pass column indexes to render a
compact projected table such as `["r", 0, [0, 2]]`.

For `b`, the renderer chooses a vertical grouped chart for two to eight ordered
time categories (dates, quarters, months, weeks, or years) with compatible
measures. Other comparisons use horizontal grouped bars with a shared axis when
units are compatible. Use `t` when continuity and rate of change are primary.

For `s`, `x` and at least one measure must be numeric-compatible and distinct.

## Alerts

```ts
type AlertTuple =
  | [level: AlertCode, content: string]
  | [level: AlertCode, title: string, content: string];

type AlertCode = "i" | "s" | "w" | "e" | "c";
```

Alert code mapping:

```text
i info
s success
w warning
e error
c critical
```

## Normalized report

Compact input is only the LLM output boundary. `agent-ui-render normalize`
expands it into readable objects (`schema=ui.input.normalized`, `version: 1`):
short keys become full names (`d` -> `datasets`, `v` -> `views`), indexes
become keys, view codes become intents (`t` -> `trend`, `r` ->
`precise_records`), type codes expand (`cur` -> `currency`), and labels are
derived. You never author this form — run
`agent-ui-render normalize <input.json> <normalized.json>` to inspect it when
debugging.

## Runtime limits

Payload size limits are trusted host/runtime policy. Configure them through the
CLI config file, not through payload fields. LLM-authored JSON must never include
limit overrides.

## Security rules

Every payload string is untrusted display text. Never include raw HTML/CSS/JS,
component/action names, styles, event handlers, `dangerouslySetInnerHTML`, or
`javascript:` URLs.

Core principle: **LLM output uses compact tuples; tools normalize into readable
objects; the planner generates canonical UI spec; renderers render only
validated normalized data.**

# Agent UI Compact Input v2

Compact v2 preserves the v1 dataset-oriented contract and adds explicit
governed chart tuples. Use it only when v1 semantic views cannot represent the
requested visualization.

```ts
type CompactReportV2 = {
  version: 2;
  t?: string;
  s?: string;
  theme?: "report-light" | "technical-dark" | "executive-clean";
  density?: "comfortable" | "compact";
  emphasis?: "strong" | "subtle";
  d?: DatasetTuple[];
  m?: MetricTuple[];
  v?: ViewV2[];
  a?: AlertTuple[];
  md?: MarkdownTuple[];
  dict?: Record<string, string[]>;
};
```

Dataset, metric, alert, markdown, dictionary, theme, density, and emphasis
shapes are unchanged from v1. Read `dataset.md` before shaping data and
`charts-v2.md` before using a v2 chart opcode.

## Example

```json
{
  "version": 2,
  "t": "Latency Distribution",
  "d": [
    [
      "latency",
      [["service", "s"], ["latency_ms", "n"]],
      [["api", 182], ["api", 205], ["worker", 311]]
    ]
  ],
  "v": [
    ["hist", 0, 1, {"bn": 20}],
    ["box", 0, 0, 1],
    ["r", 0]
  ]
}
```

## Rules

- Author compact tuples, never normalized reports or Vega-Lite specs.
- Define every dataset once under `d`.
- Reference datasets and columns by index.
- Keep values source-faithful; do not invent samples or statistical summaries.
- Raw Vega-Lite, external data URLs, arbitrary transforms, expressions,
  styles, colors, images, isotypes, SVG, and maps are forbidden.
- Run `agent-ui-render validate --warnings-as-errors` before delivering.
- Render rich charts with `render html`; `render static-html` provides native
  charts or an accessible fact-preserving table fallback.

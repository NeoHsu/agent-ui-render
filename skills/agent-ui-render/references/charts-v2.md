# Compact v2 Chart Contract

Use compact input `version: 2` when a report needs an explicit chart family
that compact v1 semantic views cannot express. Vega-Lite is an internal
renderer only: never author raw Vega-Lite JSON.

## Boundary

- Put rows once under top-level `d`.
- Chart tuples reference datasets and columns by zero-based indexes.
- Options are optional and always last.
- Do not embed rows, Vega-Lite specs, transforms, expressions, colors, URLs,
  HTML, SVG, CSS, JavaScript, image data, icons, or maps in a chart tuple.
- Image, isotype, `geoshape`, and every geographic chart are unsupported.

Notation: `D` dataset index, `X/Y` column index, `Ys` measure indexes, `C`
group/color index, `S` size index, `O` options.

## Chart tuples

<!-- markdownlint-disable MD013 -->

```ts
type ChartView =
  | ["ln", D, X, Ys, O?]
  | ["ar", D, X, Ys, O?]
  | ["tr", D, X, Y, width: number, C?, O?]
  | ["reg", D, X, Y, C | null, "linear" | "loess", O?]
  | ["bar", D, X, Ys, O?]
  | ["gantt", D, task: number, start: number, end: number, group?: number, O?]
  | ["water", D, X, Y, O?]
  | ["bullet", D, category: number, actual: number, target: number, ranges?: number[], O?]
  | ["range", D, category: number, start: number, end: number, O?]
  | ["hist", D, X, O?]
  | ["den", D, X, C?, O?]
  | ["dot", D, X, Y?, O?]
  | ["box", D, category: number, value: number, O?]
  | ["box5", D, category: number, min: number, q1: number, median: number, q3: number, max: number, O?]
  | ["qq", D, X, Y?, O?]
  | ["sc", D, X, Y, C?, S?, O?]
  | ["par", D, dimensions: number[], C?, O?]
  | ["tri", D, a: number, b: number, c: number, label?: number, O?]
  | ["heat", D, X, Y, value?: number, O?]
  | ["mos", D, X, Y, value: number, O?]
  | ["pie" | "don" | "rad", D, category: number, value: number, O?]
  | ["err" | "band", D, X, lower: number, upper: number, center?: number, O?]
  | ["candle", D, X, open: number, high: number, low: number, close: number, volume?: number, O?]
  | ["text", D, X, Y, text: number, O?]
  | ["tick", D, X, Y?, O?]
  | ["rule", D, x1: number, x2: number, y1: number, y2: number, O?];
```

<!-- markdownlint-enable MD013 -->

Examples:

```json
["hist", 0, 1, {"bn": 30}]
["ar", 0, 0, [1, 2], {"st": "zero"}]
["heat", 0, 0, 1, 2]
["box", 0, 0, 1]
["sc", 0, 0, 1, 2, 3]
["candle", 0, 0, 1, 2, 3, 4]
```

## Options

<!-- markdownlint-disable MD013 -->

| Key | Values |
| --- | --- |
| `t` | safe chart title string |
| `or` | `h`, `v` |
| `st` | `none`, `zero`, `normalize`, `center` |
| `ag` | `sum`, `mean`, `median`, `min`, `max`, `count` |
| `bn` | positive integer or `[xBins, yBins]` |
| `ip` | `linear`, `monotone`, `step`, `step-before`, `step-after` |
| `mode` | `slope`, `bump`, `stream`, `horizon`, `diverging`, `strip`, `rug`, `pyramid` |
| `pt`, `lb`, `lg`, `tip`, `zero`, `jitter` | boolean |
| `sort` | `asc`, `desc`, `none` |
| `top` | positive integer |
| `shape` | `circle`, `square`, `tick` |
| `sel` | `none`, `hover`, `click`, `brush`, `zoom`, `legend` |
| `resolve` | `shared`, `independent` |

<!-- markdownlint-enable MD013 -->

Unknown options are validation errors. Not every option applies to every chart;
use only options that express an actual requirement.

Rich HTML always provides structured SVG mark tooltips and pointer feedback.
`sel` adds a visible governed interaction: `hover` emphasizes the nearest mark;
`click`, `brush`, and `legend` dim unselected marks; `zoom` binds pan/zoom to
scales and exposes minus/plus controls. Persistent interactions include Clear
and Escape handling. Static HTML remains
non-interactive and preserves facts through native output or a table fallback.

## Layout tuples

```ts
type LayoutView =
  | ["layer", ChartView[], O?]
  | ["facet", ChartView, row: number | null, column?: number | null, O?]
  | ["concat", "h" | "v", Array<ChartView | LayerView>, O?]
  | ["repeat", ChartView, columns: number[], "h" | "v" | "grid", O?];
```

Examples:

```json
["layer", [["sc", 0, 0, 1], ["reg", 0, 0, 1, null, "linear"]]]
["facet", ["ln", 0, 0, [1]], 2, null]
["concat", "h", [["bar", 0, 0, [1]], ["sc", 0, 1, 2]]]
```

## Type compatibility

Numeric roles require `n`, `cur`, or `pct` columns. Temporal axes should use
`d` or `dt`. Category/group/text channels may use `s`, `b`, date/time, or
dictionary-backed strings. Every reference must exist in the selected
materialized dataset.

For simple trend, comparison, distribution, composition, relationship, and
records views, the v1-compatible `t`, `b`, `d`, `p`, `s`, and `r` tuples remain
valid in version 2 and are usually more concise.

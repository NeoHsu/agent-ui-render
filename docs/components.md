# Components and Styles

This catalog maps every compact payload field to the UI block it renders, and
documents what the presentation options actually change. Payload authors pick
facts and intent; every visual below is renderer-controlled and cannot be
styled from the payload.

## Report layout order

Both the rich preview (`render html`) and the static fallback
(`render static-html`) compose blocks in a fixed order:

```text
+------------------------------------------+
| Report header        t (title) s (summary)|
+------------------------------------------+
| Alert list           a                    |
+------------------------------------------+
| Metric grid          m                    |
+------------------------------------------+
| Key insights         i                    |
+------------------------------------------+
| Markdown sections    md                   |
+------------------------------------------+
| Views (tables/charts) v + d               |
+------------------------------------------+
| Assumptions          as                   |
+------------------------------------------+
| Footer               (fixed)              |
+------------------------------------------+
```

## Component catalog

| Component | Payload source | Appearance |
| --- | --- | --- |
| Report header | `t`, `s` | Full-width gradient banner with the title as `h1` and the summary as supporting text. Theme controls the gradient. |
| Alert list | `a` | One card per alert with level-colored border, background, and text: `i` info (blue), `s` success (green), `w` warning (amber), `e` error (red), `c` critical (strong red). Optional bold title above the content. |
| Metric grid | `m` | Responsive card row. Each card shows the label, the formatted value, and, when the tuple carries a delta, a derived delta label such as `+12.5%` or `-3` under the value. |
| Key insights | `i` | A card titled "Key insights" holding a bullet list of takeaway strings, placed before the views. |
| Markdown sections | `md` | One card per section with optional heading. Restricted markdown subset plus `{critical: ...}`-style semantic tokens; raw HTML never renders. |
| Data table | view `r` | Striped table with formatted cells. In the rich preview, status-like short strings (for example `failed`, `confirmed`) in status-like columns render as colored badges. |
| Line chart | view `t` | Multi-series line chart with axis ticks and legend. |
| Bar chart | views `b`, `d` | Horizontal grouped bars by default; `b` switches to vertical grouped bars for 2–8 ordered time categories (dates, quarters, months, weeks, years) with unit-compatible measures. |
| Pie chart | view `p` | Pie with legend for up to 5 positive categories; otherwise falls back to a bar chart. |
| Scatter chart | view `s` | Point chart over two numeric columns. |
| Advanced charts | v2 chart tuples | Vega-Lite SVG with a toolbar lane, structured tooltips, and optional `sel` interactions. See `docs/charts-v2.md`. |
| Assumptions | `as` | A muted card titled "Assumptions and limitations" near the footer. |
| Footer | none | Fixed one-line provenance note. |

Blocks whose payload field is absent are omitted entirely.

## Presentation profile

The three payload-selectable options map to `data-theme`, `data-density`, and
`data-emphasis` attributes that drive renderer CSS. Hosts can override colors
through trusted config theme tokens (see the README), never through the
payload.

| Option | Values | Effect |
| --- | --- | --- |
| `theme` | `report-light` (default) | Neutral light report styling. |
| | `technical-dark` | Dark dashboard surfaces, brighter series palette, dark code blocks; suits incident and operations reports. |
| | `executive-clean` | White cards, blue gradient header, restrained shadows; suits finance and leadership briefs. |
| `density` | `comfortable` (default) | Roomy paddings and card spacing for narrative reports. |
| | `compact` | Tighter paddings, smaller radii, denser metric cards for multi-view dashboards. |
| `emphasis` | `strong` (default) | Full-intensity alert and callout colors. |
| | `subtle` | Softens alert intensity: critical drops to error-level colors and warning/info backgrounds lighten. |

## Value formatting

Column type codes and metric formats control cell and metric rendering:

| Type code | Rendered as |
| --- | --- |
| `cur` | Currency; a 3-letter unit such as `USD` renders as a symbol in the rich preview (`$135,000`) and as a `USD 135000` prefix in static HTML. |
| `pct` | Ratio times 100 with a percent sign: `0.125` renders near `12.5%`. |
| `n` | Plain number, up to 2 decimal places. |
| `d`, `dt` | Date and datetime strings render as provided. |
| `b` | `true` / `false`. |
| `null` cell | Em dash `—`. |

Metric deltas format the same way: delta format `pct` renders `+12.5%`, delta
format `n` or a bare number renders `+3400` or `-3`, and a zero delta renders
without a sign.

## Rich preview versus static fallback

| Behavior | `render html` | `render static-html` |
| --- | --- | --- |
| Charts | Vue/Vega-Lite SVG with tooltips and interactions | Native Rust SVG for v1 views; v2 charts fall back to an explanation plus projected table |
| Status badges | Colored badges for status-like table cells | Plain formatted text |
| JavaScript | Required (embedded bundle) | None |
| Facts shown | Identical validated normalized data on both paths | |

## Source of truth

Visual behavior is maintained in `renderer-vue/src` (components, `agent-ui.css`,
`format.ts`, `chart-selection.ts`) and mirrored for no-JS output in the Rust
`render` module. When this document and the renderer disagree, the renderer
wins; update this catalog in the same change.

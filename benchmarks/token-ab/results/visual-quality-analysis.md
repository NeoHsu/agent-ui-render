# Visual-quality analysis: direct HTML vs agent-ui-render

- Date: 2026-07-10
- Source: formal `claude-sonnet-5` benchmark, 10 cases × 3 repetitions

## Executive finding

The user's observation is supported by a review of the paired screenshots:
direct HTML generally has stronger art direction, chart readability, and
report-specific composition. The original benchmark's 100% acceptance result
does not contradict this finding because its checks verified facts, required
structures, syntax, and browser rendering—not visual quality.

The gap can be reduced substantially without giving up compact payloads, but the
highest-impact work belongs in the deterministic renderer rather than in a much
larger Skill prompt. The Skill already encoded the requested data and view
intent correctly in most cases; the renderer often discarded or under-presented
that intent.

## Evidence from the formal runs

<!-- markdownlint-disable MD013 -->

| Observation | Measured evidence | Visual effect |
| --- | ---: | --- |
| Design controls were never selected | `theme`, `density`, and `emphasis` omitted in 30/30 payloads | Every report uses the same default visual voice |
| View titles were generic | 85/85 compact views normalize without a title | Headings such as `Trend 1`, `Comparison 2`, and `Records 3` |
| Multi-measure bars lose data | 6/18 bar views requested two measures, while `barChartModel` renders only the first | Spend vs pipeline and stock vs reorder point become single-series charts |
| Charts lack visible context | 22 line/scatter views use no visible x labels, y ticks, or grid labels | Trends and relationships are harder to interpret at a glance |
| Scatter labels are tooltip-only | All six scatter views render unlabeled dots | Service/channel identity is invisible in a static report |
| Mixed scales share one y extent | At least three runs combined unlike units or scales | Error-rate/resolution-hour lines collapse near the baseline |
| Currency uses plain number + code | 24 currency metrics render through generic number formatting | `248000 USD` instead of a polished `$248,000`-style display |
| Layout is one full-width card per view | All view blocks are emitted as a vertical sequence | Longer, repetitive reports with less editorial grouping |

<!-- markdownlint-enable MD013 -->

The compact payloads themselves did use useful semantic intent: 46 chart views
and 39 table views were produced, including multi-measure requests. This is why
renderer improvements can raise quality without increasing model output.

## Why direct HTML looks better

1. **It adapts art direction to the subject.** Incident, reliability, delivery,
   and support reports commonly chose a dark operational dashboard; financial
   and inventory reports chose lighter editorial layouts.
2. **It writes report-specific headings.** Examples include “Monthly Revenue
   Trend”, “Stock vs. Reorder Point”, and “Request Volume vs. P95 Latency” rather
   than intent names plus sequence numbers.
3. **It spends output tokens on chart semantics.** Direct charts include axis
   labels, grid lines, point/category labels, data labels, legends, and grouped
   series.
4. **It composes a story.** Related charts are grouped, tables are visually
   subordinate, and alerts or narrative are placed according to report context.
5. **It can make one-off design choices.** Gradient headers, tailored palettes,
   annotations, and small multiples are unconstrained by a canonical renderer.

## What is already better with agent-ui-render

- Consistent tables, status badges, alert semantics, and metric-card structure.
- Deterministic escaping, validation, responsive behavior, and print handling.
- Stable theme tokens and a compact, safe authoring boundary.
- Much lower model output and measured API cost.

The target should therefore be **bespoke-looking deterministic templates**, not
reintroducing arbitrary model-authored CSS or JavaScript.

## Recommended roadmap

### P0 — semantic and chart correctness

These changes have the best quality-to-risk ratio and require no payload growth.

1. Render every requested bar measure as grouped or clustered series instead of
   silently taking the first measure.
2. Format numbers with locale grouping and render recognized currencies with
   `Intl.NumberFormat` while preserving unknown units.
3. Derive meaningful view titles from intent, dataset id, dimensions, and
   measure labels; reserve numeric fallback titles for missing metadata.
4. Add x-category labels, y ticks, grid lines, point markers, and accessible
   series labels to line charts.
5. Add visible labels to scatter points using the first categorical column when
   available.
6. Detect incompatible scales/units and render small multiples or normalized
   facets instead of one misleading shared axis.

### P1 — composition and visual hierarchy

1. Place compatible chart cards in a responsive two-column grid while keeping
   records tables full width.
2. Add compact chart-card variants and reduce repetitive nested borders.
3. Improve header variants so the three existing themes have visibly distinct
   editorial, executive, and operational identities.
4. Remove generic visible copy such as “Structured report” where it does not add
   information.
5. Give alerts, narrative, and high-priority views stronger placement rules.

### P2 — small Skill/contract improvements

Keep additions narrow so visual quality does not erase token savings.

1. Tell the Skill to select one existing theme deliberately:
   `technical-dark` for operational/incident contexts, `executive-clean` for
   executive/financial briefs, and `report-light` for general analysis.
2. Select `compact` density for multi-view dashboards and `comfortable` for
   narrative reports.
3. Avoid combining measures with incompatible units in one trend request until
   the renderer can facet them automatically.
4. Add compact layout/title hints only if renderer-derived defaults remain
   insufficient. Do not add arbitrary style, class, or component fields.

A local re-render of existing incident, marketing, finance, and support payloads
with deliberate theme/density fields confirmed that theme selection alone makes
the outputs feel closer to direct HTML, but it does not solve generic titles,
missing series, or weak chart labeling.

## Evaluation plan for the next iteration

The next benchmark should separate functional acceptance from visual quality.

1. **Renderer-only ablation:** render the exact existing compact payloads with
   the old and new renderer. This isolates renderer improvements and costs no
   model tokens.
2. **Blind screenshot comparison:** randomize A/B labels and score information
   hierarchy, chart readability, visual cohesion, domain appropriateness, and
   data density on a 1–5 rubric.
3. **Objective visual-semantic checks:** reject generic numbered headings,
   verify every requested measure appears in chart labels, require visible
   category/axis labels, and detect mixed-unit shared scales.
4. **Full paired rerun:** compare improved Skill/renderer against direct HTML,
   retaining output-token, total-token, cost, latency, repair, and acceptance
   measurements.
5. **Human review remains decisive:** aesthetics should not be collapsed into
   the existing browser smoke test.

## Expected ceiling

A canonical renderer can plausibly match or exceed direct HTML for consistency,
legibility, and common dashboard/report patterns. Direct HTML will retain a
higher ceiling for one-off art direction unless the compact contract gains more
layout primitives. The recommended path closes the largest visible gap while
preserving the project's safety and token-efficiency advantages.

# Formal token-efficiency validation

- Date: 2026-07-10
- Project commit: `ca173c9` (`v0.1.0`)
- Model: `claude-sonnet-5`, effort `medium`

## Design

- 10 report cases across simple, medium, and complex workloads.
- 3 repetitions per case and configuration.
- 30 paired comparisons / 60 total runs.
- Fresh Claude CLI session for every run.
- Identical source facts and acceptance requirements in each pair.
- Direct arm produced self-contained HTML/CSS/JS.
- agent-ui-render arm received the complete Skill plus both references, produced
  compact JSON, then passed strict CLI validation and rendering.
- Both arms were allowed one repair call.
- Acceptance checks covered artifact shape, complete source facts, external
  dependencies or strict CLI validation, requested structures, and a headless
  Chrome render.

## Results

| Metric | Direct HTML | agent-ui-render | Difference |
| --- | ---: | ---: | ---: |
| Mean effective input tokens | 1,313 | 8,566 | +7,252 |
| Mean output tokens | 5,184 | 344 | -4,840 |
| Mean total tokens | 6,497 | 8,910 | +2,413 |
| Mean API cost | $0.0817 | $0.0098 | -$0.0718 |
| Mean model duration | 37.25 s | 3.82 s | -33.43 s |
| Fully valid artifacts | 30/30 | 30/30 | equal |

Paired bootstrap results:

- Output-token savings: **93.4%** (95% CI: 92.4% to 94.2%).
- Total-token savings: **-37.1%** (95% CI: -53.5% to -21.9%); in other
  words, total token volume increased by 37.1% because of Skill/reference input.
- Actual API-cost savings: **88.0%** (95% CI: 85.3% to 89.9%).
- Every eval reduced output tokens; per-eval savings ranged from 90.0% to 95.3%.
- Direct HTML and agent-ui-render each needed one repair call.
- Total formal benchmark API cost was $2.7441.

## Interpretation

The evidence supports the narrow claim that agent-ui-render substantially
reduces **LLM output tokens**. It does not support a claim that it reduces the
raw **input + output token count** when the full Skill and references are loaded
for every call. Despite the larger total token count, actual cost was lower
because the approach replaces expensive generated markup with compact output,
and repeated Skill context benefits from prompt caching.

CLI-generated HTML bytes are excluded from model output tokens. API usage comes
directly from Claude CLI result envelopes rather than character-count estimates.

## Raw artifacts

Raw responses, API usage, validation evidence, grading files, screenshots,
aggregate JSON, and the static review viewer are stored at:

```text
target/token-ab/formal-sonnet-5/iteration-1/
```

Open `review.html` to inspect paired qualitative outputs and formal grades.

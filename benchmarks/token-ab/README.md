# Token A/B benchmark

This benchmark compares two ways of producing the same self-contained report:

- `without_skill`: Claude directly authors complete HTML/CSS/JS.
- `with_skill`: Claude receives the full `agent-ui-render` skill and references,
  authors compact JSON, and the native CLI validates and renders it.

The formal suite contains 10 paired cases at three complexity levels and runs
three repetitions per configuration (60 configurations total). Both sides use
the same model, effort, source facts, fresh context, repair limit, and browser
acceptance checks.

## Run

Prerequisites:

- Authenticated `claude` CLI
- `target/release/agent-ui-render`
- Google Chrome at the default macOS path (override with `--chrome`)

```bash
python3 benchmarks/token-ab/run.py
```

Useful options:

```bash
# One-case smoke run
python3 benchmarks/token-ab/run.py --case subscription-kpis --repetitions 1

# Resume or change concurrency
python3 benchmarks/token-ab/run.py --max-workers 4
```

Runs are resumable. Raw responses, exact API usage, validation evidence,
screenshots, grading, and aggregate reports are written under:

```text
target/token-ab/formal-sonnet-5/iteration-1/
```

## Metrics

The report separates:

- main-model input and output tokens;
- cache creation/read tokens;
- all-model tokens reported by Claude Code;
- API cost;
- automated acceptance pass rate;
- cost per fully valid artifact;
- paired savings with bootstrap 95% confidence intervals.

CLI-generated HTML size is intentionally excluded from model output tokens. The
`with_skill` input total includes the full skill and both bundled references on
every uncached invocation.

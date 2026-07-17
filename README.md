# agent-ui-render

`agent-ui-render` is a native CLI for agent-authored UI previews. Coding agents
write compact, governed JSON; the CLI validates, normalizes, plans, and renders
that data into browser-openable HTML, no-JS static HTML, or a Vue handoff bundle.

The runtime is Rust. The visual renderer is authored in Vue during development
and embedded into the release binary, so installed users do **not** need Node,
Bun, npm, Vue, or `node_modules`.

## What this project is for

- Give coding agents a small, safe UI output contract instead of asking them to
  hand-write HTML, CSS, Vue, React, or JavaScript.
- Convert compact model-authored `version: 1` semantic reports and `version: 2`
  advanced chart payloads into validated runtime data before anything is rendered.
- Produce portable preview artifacts for reports, tables, metric cards, charts,
  alerts, and markdown narrative.
- Keep release artifacts as single native binaries plus installer scripts.

## Why compact data instead of direct HTML?

Anthropic's [Using Claude Code: The unreasonable effectiveness of
HTML][claude-html] makes a point this project agrees with: HTML is a useful
artifact boundary for agent work because it is portable, inspectable, and
immediately useful in a
browser.

`agent-ui-render` optimizes a different part of that workflow. Having an LLM
agent directly author full HTML, CSS, and JavaScript is expressive, but for
report-like UIs it spends expensive output tokens on repeated tags, styles,
boilerplate, and layout code. Output tokens are also typically priced higher than
input tokens, so repeating UI markup is a poor place to spend model budget when
the underlying result is mostly data plus visual intent.

This project keeps the useful HTML artifact, but moves verbose rendering into a
tool:

```text
+----------------------------+       compact data       +------------------+
| LLM agent                  | -----------------------> | agent-ui-render  |
| facts + visual intent      |                          | validate/render  |
+----------------------------+                          +--------+---------+
                                                                  |
                                                                  v
                                                        +------------------+
                                                        | HTML artifact    |
                                                        | browser-ready    |
                                                        +------------------+
```

The model emits a compact dataset-oriented payload; the trusted CLI expands it
into validated runtime data and renders the HTML. That reduces output-token
usage, centralizes rendering and safety rules, and gives agents a smaller,
repeatable contract for charts, tables, metrics, alerts, and markdown narrative.

## Measured token and cost efficiency

A formal paired benchmark used `claude-sonnet-5` at medium effort across 10
report cases, with three repetitions per configuration: 30 direct-HTML runs and
30 `agent-ui-render` runs. Both configurations produced 30/30 artifacts that
passed the same fact-preservation, structure, and headless-browser checks.

![Mean Claude token composition per accepted artifact][benchmark-token-mix]

The compact payload reduced mean model output from 5,184 to 344 tokens, a
**93.4% reduction** (95% CI: 92.4% to 94.2%). The full Skill and reference
context increased effective input tokens, so raw input-plus-output volume rose
37.1%. This project therefore claims lower **model output**, not lower total
context volume.

![Output-token savings by report complexity][benchmark-complexity]

Actual measured API cost fell from $0.0817 to $0.0098 per accepted artifact,
an **88.0% reduction** (95% CI: 85.3% to 89.9%). These costs include prompt
caching and the single repair call required by each configuration.

![Mean measured API cost per accepted artifact][benchmark-cost]

The benchmark also reduced mean model duration from 37.25 seconds to 3.82
seconds. See the [formal benchmark report][benchmark-report] for the complete
method, caveats, confidence intervals, and reproducible harness.

## Theme showcase

The same compact payload can render as different host-selected themes without
asking the agent to rewrite the UI.

| Report Light | Executive Clean | Technical Dark |
| --- | --- | --- |
| ![Report Light][shot-light] | ![Executive Clean][shot-clean] | ![Technical Dark][shot-dark] |

The showcase payload lives at
`docs/assets/screenshots/markdown-brief.input.json`; regenerate any theme shot
by switching its `theme` value, rendering HTML, and capturing a 1600x1800
headless-browser screenshot.

## Install

### Installer script

```bash
base="https://github.com/NeoHsu/agent-ui-render/releases/latest/download"
curl --proto '=https' --tlsv1.2 -LsSf \
  "$base/agent-ui-render-installer.sh" | sh
```

### Windows PowerShell

```powershell
$base = "https://github.com/NeoHsu/agent-ui-render/releases/latest/download"
irm "$base/agent-ui-render-installer.ps1" | iex
```

### Direct release downloads

| Platform | Asset |
| --- | --- |
| Apple Silicon macOS | `agent-ui-render-aarch64-apple-darwin.tar.xz` |
| Intel macOS | `agent-ui-render-x86_64-apple-darwin.tar.xz` |
| ARM64 Linux | `agent-ui-render-aarch64-unknown-linux-gnu.tar.xz` |
| x64 Linux | `agent-ui-render-x86_64-unknown-linux-gnu.tar.xz` |
| x64 Windows | `agent-ui-render-x86_64-pc-windows-msvc.zip` |

Each archive contains the prebuilt `agent-ui-render` executable plus release
metadata. Checksums are published next to the release assets, and each platform
archive has GitHub build-provenance attestation. Verify a download before use:

```bash
gh attestation verify <archive> --repo NeoHsu/agent-ui-render
```

### mise

```bash
mise use -g github:NeoHsu/agent-ui-render
```

In `mise.toml`:

```toml
[tools]
"github:NeoHsu/agent-ui-render" = "latest"
```

### From source

From the published repository:

```bash
cargo install --git https://github.com/NeoHsu/agent-ui-render agent-ui-render
```

From a local checkout:

```bash
mise install
make setup
make generate
cargo install --path crates/agent-ui-render-cli
```

Verify the install:

```bash
agent-ui-render --version
agent-ui-render --help
```

## Agent Skill

Install the bundled AI agent skill so coding agents know how to author compact
Agent UI payloads instead of hand-writing HTML, CSS, JavaScript, or arbitrary UI
components.

From the published repository:

```bash
npx skills add NeoHsu/agent-ui-render --skill agent-ui-render
```

From a local checkout of this repo:

```bash
npx skills add . --skill agent-ui-render
```

For non-interactive setup across all supported agents, add `--agent '*' -y`:

```bash
npx skills add . --skill agent-ui-render --agent '*' -y
```

Use `--copy` if you want the installed skill to be a standalone copy instead of
a symlink back to the repo checkout:

```bash
npx skills add . --skill agent-ui-render --copy
```

The CLI install gives agents a renderer; the skill install gives agents the
payload contract and safety rules.

## Quick render

Create a compact input file:

```bash
cat > /tmp/revenue.input.json <<'JSON'
{
  "version": 1,
  "t": "Revenue Overview",
  "d": [
    [
      "sales",
      [["month", "s"], ["revenue", "cur", "USD"]],
      [["Jan", 120000], ["Feb", 135000]]
    ]
  ],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
JSON
```

Validate and render it:

```bash
agent-ui-render validate /tmp/revenue.input.json
agent-ui-render render html /tmp/revenue.input.json /tmp/revenue.html
agent-ui-render render static-html /tmp/revenue.input.json /tmp/revenue.static.html
```

Open `/tmp/revenue.html` in a browser.

## Runtime flow

```text
+------------------------------+
| Compact payload (v1 or v2)   |
+---------------+--------------+
                |
                v
+------------------------------+
| agent-ui-render validate     |
| - structure and references   |
| - unsafe content and limits  |
+---------------+--------------+
                |
                v
+------------------------------+
| Normalize to domain::Report  |
+---------------+--------------+
                |
        +-------+-------+----------------+
        |               |                |
        v               v                v
+---------------+ +-------------+ +----------------+
| plan ui.spec  | | render html | | render vue     |
| JSON          | | or static   | | handoff bundle |
+---------------+ +-------------+ +----------------+
```

## Compact input shape

```json
{
  "version": 1,
  "t": "Revenue Overview",
  "d": [["sales", [["month", "s"], ["revenue", "cur", "USD"]], [["Jan", 120000]]]],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
```

The compact wire format uses dataset indexes and short view/alert codes to keep
LLM output small. Compact code mappings live in `wire::compact`; `normalize`
expands them into the clean `domain::Report` model with
`schema: "ui.input.normalized"` with the matching input version before planning
or rendering. Version 2 adds governed chart opcodes backed internally by
Vega-Lite; raw Vega-Lite specs, images, isotypes, and maps are not accepted.

## CLI surface

```bash
agent-ui-render validate <input.json>
agent-ui-render normalize <input.json> [output.json]
agent-ui-render plan <input.json> [output.json]
agent-ui-render render html <input.json> <output.html>
agent-ui-render render static-html <input.json> <output.html>
agent-ui-render render vue <input.json> <output.vue>
agent-ui-render schema print <compact|compact-v2|normalized|normalized-v2|spec|spec-v2|config>
agent-ui-render completion <shell>
```

Global flags:

```bash
-o, --output <human|json>
--warnings-as-errors
--quiet
--pretty
--config <path>
```

## Configurable limits and theme colors

Use an explicit config file to override runtime guardrails and trusted host color
tokens:

```bash
agent-ui-render --config agent-ui-render.config.json render html input.json report.html
```

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
    "primary": "#8b5cf6",
    "series1": "#8b5cf6",
    "series2": "#06b6d4"
  }
}
```

Limits and theme tokens are host/runtime policy and are never read from the
untrusted payload. Theme tokens map to `--agent-*` CSS custom properties and are
validated as safe CSS color literals before render output is written.

## Development from source

```bash
make setup      # install renderer dependencies
make generate   # build generated/renderer.js and generated/renderer.css
make test       # run renderer and Rust tests
make msrv       # check all Rust targets with Rust 1.91
make workflow-check   # validate workflow syntax and Action pins
make performance-check # enforce renderer raw/gzip budgets
make dev              # run the CLI help from source
make check            # run release-quality checks
```

Build/runtime architecture:

```text
+--------------------------+     bun + Vite      +-------------------------+
| renderer-vue/src         | ------------------> | generated/renderer.*    |
| Vue + CSS + TS sources   |                     | JS/CSS release assets   |
+--------------------------+                     +------------+------------+
                                                               |
                                                               | include_str!
                                                               v
                                                  +-------------------------+
                                                  | agent-ui-render binary  |
                                                  | no Node/Bun at runtime  |
                                                  +-------------------------+
```

## Documentation map

- `docs/usage.md` - consumer workflow for creating, validating, and rendering
  compact payloads.
- `docs/agent-reference.md` - task routing for coding agents using or modifying
  this repository.
- `docs/development.md` - maintainer setup, change maps, and verification.
- `docs/troubleshooting.md` - common failures and recovery steps.
- `docs/architecture.md` - runtime, build-time, and source-of-truth model.
- `docs/cli-reference.md` - command reference and exit codes.
- `docs/compatibility.md` - versioning and contract-change policy.
- `docs/charts-v2.md` - governed advanced chart coverage and v2 behavior.
- `docs/components.md` - component catalog and presentation style reference.
- `SECURITY.md` - private vulnerability reporting and release verification.
- `docs/security-model.md` - trust boundaries and unsafe-content rules.
- `docs/renderer-development.md` - Vue renderer development and handoff bundle.
- `docs/release.md` - release process and cargo-dist publishing flow.
- `skills/agent-ui-render/references/ui-input.md` - compact payload contract.
- `skills/agent-ui-render/references/dataset.md` - dataset tuple rules.
- `schemas/v1/*.schema.json` and `schemas/config.schema.json` - JSON Schema
  mirrors for integration checks.

[claude-html]: https://claude.com/blog/using-claude-code-the-unreasonable-effectiveness-of-html
[benchmark-report]: benchmarks/token-ab/results/formal-sonnet-5.md
[benchmark-token-mix]: docs/assets/benchmarks/token-composition.svg
[benchmark-complexity]: docs/assets/benchmarks/output-savings-by-complexity.svg
[benchmark-cost]: docs/assets/benchmarks/api-cost.svg
[shot-light]: docs/assets/screenshots/markdown-brief-report-light.png
[shot-clean]: docs/assets/screenshots/markdown-brief-executive-clean.png
[shot-dark]: docs/assets/screenshots/markdown-brief-technical-dark.png

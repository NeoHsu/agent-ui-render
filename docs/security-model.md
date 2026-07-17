# Security Model

`agent-ui-render` treats all payload strings as untrusted data.

## Trust boundaries

```text
UNTRUSTED INPUT                                TRUSTED RUNTIME

+-------------------------+       +----------------------------------+
| Model/tool/user compact |       | CLI binary                       |
| JSON payload            |       | bundled assets, schemas, config  |
+------------+------------+       +----------------+-----------------+
             |                                     |
             v                                     v
+-------------------------+       +----------------------------------+
| Parse + validate        | <---- | Host policy                      |
| unsafe-content checks   |       | limits and safe color tokens     |
+------------+------------+       +----------------------------------+
             |
     +-------+--------+
     | reject invalid |
     | or unsafe data |
     +-------+--------+
             |
             v
+-------------------------+
| Normalized report data  |
+------------+------------+
             |
      +------+------+
      |             |
      v             v
+-------------+ +------------------------+
| Plan ui.spec | | Render escaped output |
+-------------+ +------------------------+
```

- Untrusted: compact JSON payloads produced by models or tools.
- Trusted: CLI binary, bundled renderer assets, schemas, and optional host config.
- Host config controls runtime limits and optional renderer theme color tokens;
  payloads cannot raise their own limits or provide CSS.

## What is allowed

Payloads may contain structured data, primitive table cells, metrics, alerts, and
safe markdown text. Markdown is parsed as a small allowlist subset:

- headings `#` through `###`
- paragraphs, blockquotes, unordered and ordered lists
- fenced code blocks
- emphasis/strong/inline code
- safe links: `https://`, `http://`, `mailto:`, `/path`, and `#anchor`
- semantic tokens like `{warning: pending}`

## What is blocked

```text
+-------------------------------+--------------------+
| Payload content               | Decision           |
+-------------------------------+--------------------+
| raw HTML, SVG, iframe, script | reject             |
| style tags or inline CSS      | reject             |
| JavaScript URLs or handlers   | reject             |
| Vue/React/JSX/template code   | reject             |
| arbitrary component/actions   | reject             |
| raw Vega-Lite specs/expressions| reject             |
| image, isotype, geoshape, map | reject             |
| safe text, markdown, data     | validate normally  |
+-------------------------------+--------------------+
```

Raw HTML, Vue, React, SVG, CSS, event handlers, component/action injection,
raw Vega-Lite JSON, external chart URLs, images, isotypes, geoshapes, maps, and
`javascript:` URLs are not part of the contract. Version 2 chart specs are
created only by the trusted Rust planner, and the browser Vega runtime uses a
loader that rejects all external resources.

## Runtime guardrails

Default limits are defined in `Limits` and can be lowered or raised by trusted
config:

- `maxInputBytes`
- `maxDatasets`
- `maxDictionaries`
- `maxDictionaryEntries`
- `maxColumnsPerDataset`
- `maxRowsPerDataset`
- `maxCellsPerDataset`
- `maxTotalRows`
- `maxTotalCells`
- `maxMetrics`
- `maxViews`
- `maxAlerts`
- `maxMarkdownSections`
- `maxStringChars`
- `maxMarkdownSectionChars`
- `maxTotalMarkdownChars`
- `maxFindings`
- `warnOutputHtmlBytes`
- `maxOutputHtmlBytes`

Input reads stop at `maxInputBytes + 1` instead of buffering unbounded stdin.
Validation caps retained diagnostics and dataset work, and fails closed for
oversized or structurally invalid payloads. Render commands warn when generated
HTML exceeds `warnOutputHtmlBytes`; with `--warnings-as-errors`, this blocks the
command. Output above `maxOutputHtmlBytes` is always rejected before writing.

Trusted config may also set `themeTokens` for renderer colors. Theme token values
are validated as safe CSS color literals before render output is written; raw CSS
remains outside the payload contract.

## Release gates

```text
+-----------------------------+
| Pull request / release      |
+-------------+---------------+
              |
              v
+-----------------------------+
| Vue typecheck, tests, build |
| generated asset drift check |
+-------------+---------------+
              |
              v
+-----------------------------+
| cargo audit + bun audit     |
+-------------+---------------+
              |
              v
+-----------------------------+
| rustfmt and clippy          |
+-------------+---------------+
              |
              v
+-----------------------------+
| Rust 1.91 MSRV check        |
+-------------+---------------+
              |
              v
+-----------------------------+
| cargo test --workspace      |
+-------------+---------------+
              |
              v
+-----------------------------+
| scripts/verify-release.sh   |
+-----------------------------+
```

CI blocks on these gates before release-sensitive changes are accepted.

## Non-goals

The renderer is not a sandbox for arbitrary UI code and does not execute payload
scripts. Hosts embedding generated HTML should still serve it with appropriate
CSP and origin isolation for their environment.

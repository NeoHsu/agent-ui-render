# Release

The release layout follows the `atla` pattern: Rust workspace, dedicated CLI and
core crates, `cargo-dist` installers, and generated Vue assets embedded before
publishing.

## Release flow

```text
+------------------------------+
| Maintainer prepares release  |
+--------------+---------------+
               |
               v
+------------------------------+
| make check passes locally    |
+--------------+---------------+
               |
               v
+------------------------------+
| generated/renderer.*         |
| committed with source        |
+--------------+---------------+
               |
               v
+------------------------------+
| Push version tag             |
+--------------+---------------+
               |
               v
+------------------------------+
| GitHub release workflow      |
+--------------+---------------+
               |
               v
+------------------------------+
| tests, MSRV, audit, smoke    |
+--------------+---------------+
               |
               v
+------------------------------+
| cargo-dist build/upload      |
+--------------+---------------+
               |
               v
+------------------------------+
| GitHub Release artifacts     |
+------------------------------+
```

## Before tagging

Preferred local command:

```bash
make check
```

Equivalent expanded commands:

```bash
cd renderer-vue
bun install
bun run typecheck
bun run test
bun run build
bun audit

cd ..
cargo audit
cargo +1.91 check --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check-workflows.sh
./scripts/verify-release.sh
```

The release binary should include `generated/renderer.js` and
`generated/renderer.css` through `include_str!`; no runtime `node_modules` are
needed. `scripts/verify-release.sh` builds the release binary, validates bundled
schemas as JSON, validates all examples, normalizes/plans an example, and renders
HTML/static HTML smoke artifacts. `make check` also enforces raw/gzip renderer
budgets and a measured browser chart-readiness threshold.

## Tag release

```bash
version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "agent-ui-render") | .version')"
git tag "v$version"
git push origin "v$version"
```

The privileged release workflow reruns Vue typechecks/tests/builds, generated
asset drift checks, Rustfmt, Clippy, Rust 1.91 compatibility, all Rust tests,
dependency audits, documentation/example checks, release smoke verification,
and browser interaction smoke before installing `cargo-dist`. Pull requests to
the release workflow build and upload every configured platform artifact without
publishing it. Tag builds publish only after all platform builds succeed.

## Release artifact expectations

```text
+-------------------------------+
| GitHub Release                |
+---------------+---------------+
                |
      +---------+---------+-------------------+
      |                   |                   |
      v                   v                   v
+-------------+   +---------------+   +----------------+
| installer   |   | installer.ps1 |   | target archives|
| macOS/Linux |   | Windows       |   | native binaries|
+-------------+   +---------------+   +--------+-------+
                                               |
                                               v
                                      +----------------+
| checksums      |
| attestations   |
| release info   |
+----------------+
```

Configured targets live in `dist-workspace.toml`. Each local platform artifact
receives a GitHub build-provenance attestation. Verify a downloaded archive with:

```bash
gh attestation verify <archive> --repo NeoHsu/agent-ui-render
```

See also `docs/compatibility.md` and `docs/security-model.md` before changing
payload contracts or security-sensitive rendering behavior.

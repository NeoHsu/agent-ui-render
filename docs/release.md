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
| verify assets, audit, smoke  |
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
bun run build

cd ..
cargo audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/verify-release.sh
```

The release binary should include `generated/renderer.js` and
`generated/renderer.css` through `include_str!`; no runtime `node_modules` are
needed. `scripts/verify-release.sh` builds the release binary, validates bundled
schemas as JSON, validates all examples, normalizes/plans an example, and renders
HTML/static HTML smoke artifacts.

## Tag release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The privileged release workflow uses the committed generated assets, runs
`cargo audit`, runs release smoke verification, installs `cargo-dist`, and
publishes native binaries and installer scripts for the configured targets.

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
                                      | attestations   |
                                      | provenance     |
                                      +----------------+
```

Configured targets live in `dist-workspace.toml`.

See also `docs/compatibility.md` and `docs/security-model.md` before changing
payload contracts or security-sensitive rendering behavior.

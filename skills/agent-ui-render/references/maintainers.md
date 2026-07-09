# Maintainer Guide

This version uses a Rust runtime and Vue build-time renderer.

## Architecture map

```text
crates/agent-ui-render-cli      clap CLI, file IO, exit codes
crates/agent-ui-render-core     wire compact input, domain report model,
                                validation, normalization, spec planning,
                                renderers, embedded assets
renderer-vue/src                Vue component renderer source
generated/renderer.js           bundled Vue client renderer embedded by Rust
generated/renderer.css          bundled CSS embedded by Rust
schemas/                        JSON Schema mirrors
examples/                       smoke and documentation inputs
```

## Invariants

- Users do not need Node/Bun/npm at runtime.
- Vue tooling is only for development/release asset generation.
- Rust validation/normalization is the runtime source of truth.
- JSON Schemas are integration mirrors.
- Generated HTML must not load external assets.
- Payload strings are untrusted text.
- Runtime limits are configured by trusted host config, never by payload data.

## Verification

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

## Contract changes

A contract change is not complete until these agree:

- Rust domain structs/constants
- Rust compact wire mappings
- Rust validators
- Rust normalizer
- planner/static renderer when applicable
- Vue renderer types/components when applicable
- `schemas/` and config schema when applicable
- `examples/`
- skill reference docs
- tests

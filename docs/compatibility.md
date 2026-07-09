# Compatibility Policy

## Stable boundary

The stable model-authored boundary is compact input `version: 1`. Public tools
should author only this wire format. Rust validation is the source of truth;
JSON Schemas mirror the same contract for integration checks.

## Versioning rules

- Additive changes that old renderers can safely ignore may stay in version 1
  only when they do not change existing field meaning.
- New compact codes, changed tuple shapes, removed fields, or changed semantics
  require a new input version.
- Normalized reports (`schema: "ui.input.normalized"`) and UI specs
  (`schema: "ui.spec"`) are runtime/tooling artifacts, not the LLM authoring
  contract.
- Schema files must stay in parity with Rust constants and validators. CI tests
  validate examples against Rust and JSON Schema and compare schema enums with
  centralized Rust mappings.

## Release requirements for contract changes

A contract change is not releasable until all of these agree:

1. `wire::compact` code mappings and tuple interpretation.
2. `domain::report` constants/structs.
3. Rust validators and normalizer.
4. JSON Schemas under `schemas/`.
5. Examples and skill reference docs.
6. Normalize/plan golden tests and schema parity tests.
7. Vue types/components when visual behavior changes.

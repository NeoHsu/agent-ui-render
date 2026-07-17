.DEFAULT_GOAL := help

.PHONY: help setup dev generate performance-check dist-smoke-check typecheck msrv lint test audit docs-check examples-check workflow-check check verify-release visual-smoke interaction-smoke clean

help: ## Show available project commands.
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*##/ {printf "%-16s %s\n", $$1, $$2}' Makefile

setup: ## Install development dependencies for the Vue renderer.
	@echo "If tools are missing, run: mise install"
	(cd renderer-vue && bun install)

dev: generate ## Build embedded assets and print the CLI help from source.
	cargo run -- --help

generate: ## Build embedded Vue renderer assets under generated/.
	(cd renderer-vue && bun run build)

performance-check: generate ## Enforce renderer bundle size budgets.
	bun scripts/check-performance-budgets.ts

dist-smoke-check: ## Self-test archive and installer verification.
	python3 scripts/smoke-dist-artifacts.py self-test

typecheck: ## Run Vue and Rust type checks.
	(cd renderer-vue && bun run typecheck)
	cargo check --workspace --all-targets

msrv: ## Check all Rust targets with the declared minimum supported Rust version.
	cargo +1.91 check --workspace --all-targets --locked

lint: typecheck ## Run formatting and clippy checks.
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test: generate ## Run renderer and Rust tests after embedded assets are current.
	(cd renderer-vue && bun run test)
	cargo test --workspace

audit: ## Run Rust and renderer dependency advisory checks.
	@command -v cargo-audit >/dev/null 2>&1 || { echo "cargo-audit not found. Install with: cargo install cargo-audit --locked"; exit 127; }
	cargo audit
	(cd renderer-vue && bun audit)

docs-check: ## Check docs/cli-reference.md against the CLI --help output.
	./scripts/check-cli-docs.sh

examples-check: ## Validate markdown payload examples in docs/ and skills/ against the CLI.
	./scripts/check-doc-examples.sh

workflow-check: ## Validate GitHub workflow syntax and immutable Action pins.
	./scripts/check-workflows.sh

check: generate performance-check dist-smoke-check audit lint msrv test docs-check examples-check workflow-check verify-release interaction-smoke ## Run the full local release-quality check suite.

verify-release: generate ## Run release binary smoke verification.
	./scripts/verify-release.sh

visual-smoke: generate ## Build visual smoke HTML artifacts under target/visual-smoke/.
	./scripts/visual-smoke.sh

interaction-smoke: ## Exercise rich chart interactions in headless Chrome.
	./scripts/interaction-smoke.sh

clean: ## Remove Rust and visual smoke build artifacts.
	cargo clean
	rm -rf target/visual-smoke

.DEFAULT_GOAL := help

.PHONY: help setup dev generate typecheck lint test audit docs-check check verify-release visual-smoke clean

help: ## Show available project commands.
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*##/ {printf "%-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Install development dependencies for the Vue renderer.
	@echo "If tools are missing, run: mise install"
	(cd renderer-vue && bun install)

dev: generate ## Build embedded assets and print the CLI help from source.
	cargo run -- --help

generate: ## Build embedded Vue renderer assets under generated/.
	(cd renderer-vue && bun run build)

typecheck: ## Run Vue and Rust type checks.
	(cd renderer-vue && bun run typecheck)
	cargo check --workspace --all-targets

lint: typecheck ## Run formatting and clippy checks.
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test: generate ## Run Rust tests after embedded assets are current.
	cargo test --workspace

audit: ## Run cargo dependency advisory checks.
	@command -v cargo-audit >/dev/null 2>&1 || { echo "cargo-audit not found. Install with: cargo install cargo-audit --locked"; exit 127; }
	cargo audit

docs-check: ## Check docs/cli-reference.md against the CLI --help output.
	./scripts/check-cli-docs.sh

check: generate audit lint test docs-check verify-release ## Run the full local release-quality check suite.

verify-release: generate ## Run release binary smoke verification.
	./scripts/verify-release.sh

visual-smoke: generate ## Build visual smoke HTML artifacts under target/visual-smoke/.
	./scripts/visual-smoke.sh

clean: ## Remove Rust and visual smoke build artifacts.
	cargo clean
	rm -rf target/visual-smoke

export SHELL := /usr/bin/env bash -Eeu -o pipefail

.DEFAULT_GOAL := help
.PHONY: help
help:  ## Display this help documents
	@grep -E '^[0-9a-zA-Z_-]+:.*?## .*$$' ${MAKEFILE_LIST} | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-40s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build:  ## Build the project
	cargo build

.PHONY: test
test:  ## Run all tests
	cargo test

.PHONY: lint
lint:  ## Run linter (clippy)
	cargo clippy -- -D warnings

.PHONY: fmt
fmt:  ## Format code
	cargo fmt

.PHONY: fmt-check
fmt-check:  ## Check code formatting
	cargo fmt -- --check

.PHONY: example
example: build  ## Build and run example (requires sudo)
	@echo "Running example (requires sudo/admin privileges)..."
	sudo ./target/debug/examples/simple_ping google.com

.PHONY: ping
ping: build  ## Build and run ping command (requires sudo)
	@echo "Running ping command (requires sudo/admin privileges)..."
	sudo ./target/debug/ping google.com

.PHONY: clean
clean:  ## Clean build artifacts
	cargo clean

.PHONY: doc
doc:  ## Generate documentation
	cargo doc --open

.PHONY: release
release:  ## Build release version
	cargo build --release
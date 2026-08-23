SHELL := bash

.DEFAULT_GOAL := help
.PHONY: help install doctor dev build frontend fmt test check validate

help: ## List available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*## "}{printf "  %-10s %s\n", $$1, $$2}'

install: ## Install uv and frontend dependencies
	uv sync --locked
	bun install --frozen-lockfile

doctor: ## Check host development tools
	uv run scripts/doctor.py

dev: ## Run the Tauri development application
	bun run tauri:dev

build: ## Build the distributable Tauri application
	bun run tauri:build

frontend: ## Build the React frontend
	bun run build

fmt: ## Format Rust sources
	cargo fmt --all

test: ## Run platform-aware tests
	uv run scripts/test.py

check: ## Run platform-aware project checks
	uv run scripts/check.py

validate: check ## Alias for check used by workstation PR gates

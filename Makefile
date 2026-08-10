SHELL := bash

.DEFAULT_GOAL := help
.PHONY: help install dev build frontend fmt test check

help: ## List available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*## "}{printf "  %-10s %s\n", $$1, $$2}'

install: ## Install frontend dependencies
	bun install --frozen-lockfile

dev: ## Run the Tauri development application
	bun run tauri:dev

build: ## Build the distributable Tauri application
	bun run tauri:build

frontend: ## Build the React frontend
	bun run build

fmt: ## Format Rust sources
	cargo fmt --manifest-path src-tauri/Cargo.toml

test: ## Run Rust tests
	cargo test --manifest-path src-tauri/Cargo.toml

check: frontend fmt test ## Run the project checks
	cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

SHELL := bash

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
NIX_RUN := $(if $(IN_NIX_SHELL),,nix develop --command )
else
NIX_RUN :=
endif

.DEFAULT_GOAL := help
.PHONY: help install doctor dev build frontend fmt test check

help: ## List available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*## "}{printf "  %-10s %s\n", $$1, $$2}'

install: ## Install uv and frontend dependencies
	$(NIX_RUN)uv sync --locked
	$(NIX_RUN)bun install --frozen-lockfile

doctor: ## Check host development tools
	$(NIX_RUN)uv run scripts/doctor.py

dev: ## Run the Tauri development application
	$(NIX_RUN)bun run tauri:dev

build: ## Build the distributable Tauri application
	$(NIX_RUN)bun run tauri:build

frontend: ## Build the React frontend
	$(NIX_RUN)bun run build

fmt: ## Format Rust sources
	$(NIX_RUN)cargo fmt --manifest-path src-tauri/Cargo.toml

test: ## Run platform-aware tests
	$(NIX_RUN)uv run scripts/test.py

check: ## Run platform-aware project checks
	$(NIX_RUN)uv run scripts/check.py

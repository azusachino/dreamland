# Spec: Dreamland Tauri rework

## Objective

Rebuild Dreamland as a Tauri 2 desktop application while preserving its
provider set, API pagination, settings, and image-download behavior.

## Commands

- Check Rust: `cargo check --workspace`
- Test Rust: `cargo test --workspace`
- Format Rust: `cargo fmt --all -- --check`
- Check TypeScript: `bun run typecheck`
- Check frontend: `bun run build`
- Run desktop app: `bun run tauri:dev`
- Check tooling: `uv run scripts/doctor.py`
- Run daily checks: `uv run scripts/check.py` or `make check`

## Project Structure

- `index.html` — Vite entry document
- `src/` — React + TypeScript frontend
- `dist/` — Vite build output loaded by Tauri
- `crates/` — core, runtime, and Yandere adapter crates
- `src-tauri/` — Tauri application and command boundary
- `docs/` — project specification, decisions, ADRs, roadmap, and references
- `scripts/` — uv-runnable daily tooling and platform-aware checks

## Code Style

Use small command functions at the Tauri boundary and keep API/config logic in
Rust modules. The frontend uses React state and `@tauri-apps/api/core` for
backend calls.

## Testing Strategy

Keep provider response deserialization tests in the Yandere adapter crate and
runtime I/O tests in the runtime crate. Add command-boundary tests only where
behavior is not already covered by module tests. Validate the frontend through
TypeScript and Vite production checks.

## Boundaries

- Always: preserve existing user-visible features and run the repository checks.
- Always: keep provider requests and runtime persistence behind Rust commands.
- Always: treat yande.re as one provider, not the universal API contract.
- Always: approve `docs/API-V1.md` before implementing provider or runtime traits.
- Ask first: new native capabilities, plugins, or unrelated UI redesign.
- Never: expose arbitrary filesystem access to the frontend or commit secrets.

## Success Criteria

- The repository has a Tauri 2 project structure and no Dioxus dependencies.
- The Rust code is organized as a workspace with isolated runtime/provider
  boundaries.
- The frontend is strict TypeScript with typed IPC wrappers.
- The frontend can load images, paginate, save settings, and download images
  through registered Tauri commands.
- Rust tests and formatting pass; the Vite production build succeeds.

# Spec: Dreamland Tauri rework

## Objective

Rebuild Dreamland as a Tauri 2 desktop application while preserving its
site set, API pagination, settings, and image-download behavior.

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
- `crates/` — core, shared Moebooru protocol, runtime, sites composition root,
  and site adapter crates
  (`dreamland-site-yandere` and `dreamland-site-konachan`
  adapters active; `dreamland-site-pixiv`/`-twitter` remain descriptor-only
  skeletons)
- `src-tauri/` — Tauri application and command boundary
- `docs/` — project specification, decisions, ADRs, roadmap, and references
- `scripts/` — uv-runnable daily tooling and platform-aware checks

## Code Style

Use small command functions at the Tauri boundary and keep API/config logic in
Rust modules. The frontend uses React state and `@tauri-apps/api/core` for
backend calls.

## Testing Strategy

Keep site response deserialization tests in each site adapter crate and
runtime I/O tests in the runtime crate. Add command-boundary tests only where
behavior is not already covered by module tests. Validate the frontend through
TypeScript and Vite production checks.

## Boundaries

- Always: preserve existing user-visible features and run the repository checks.
- Always: keep site requests and runtime persistence behind Rust commands.
- Always: treat yande.re as one site, not the universal API contract.
- Always: approve `docs/API-V1.md` before implementing site capabilities or
  runtime traits.
- Ask first: new native capabilities, plugins, or unrelated UI redesign.
- Never: expose arbitrary filesystem access to the frontend or commit secrets.

## Success Criteria

- The repository has a Tauri 2 project structure. (The pre-Tauri Dioxus
  migration is long complete; that criterion is retired.)
- The Rust code is organized as a workspace with isolated runtime/site
  boundaries.
- The frontend is strict TypeScript with typed IPC wrappers.
- The frontend can load images, paginate, save settings, and download images
  through registered Tauri commands.
- Rust tests and formatting pass; the Vite production build succeeds.

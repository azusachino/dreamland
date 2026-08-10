# Spec: Dreamland Tauri rework

## Objective

Rebuild Dreamland as a Tauri 2 desktop application while preserving its
provider set, API pagination, settings, and image-download behavior.

## Commands

- Check Rust: `cargo check --manifest-path src-tauri/Cargo.toml`
- Test Rust: `cargo test --manifest-path src-tauri/Cargo.toml`
- Format Rust: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- Check frontend: `bun run build`
- Run desktop app: `bun run tauri:dev`
- Check tooling: `uv run scripts/doctor.py`
- Run daily checks: `uv run scripts/check.py` or `make check`

## Project Structure

- `index.html` — Vite entry document
- `src/` — React frontend
- `dist/` — Vite build output loaded by Tauri
- `src-tauri/` — Tauri application and Rust commands
- `tasks/` — rework plan and acceptance checklist
- `scripts/` — uv-runnable daily tooling and platform-aware checks

## Code Style

Use small command functions at the Tauri boundary and keep API/config logic in
Rust modules. The frontend uses React state and `@tauri-apps/api/core` for
backend calls.

## Testing Strategy

Keep the existing API deserialization tests in the Tauri crate. Add command
boundary tests only where behavior is not already covered by module tests.
Validate the frontend through Vite's production build.

## Boundaries

- Always: preserve existing user-visible features and run the repository checks.
- Always: keep provider requests and runtime persistence behind Rust commands.
- Always: treat yande.re as one provider, not the universal API contract.
- Always: approve `docs/API-V1.md` before implementing provider or runtime traits.
- Ask first: new native capabilities, plugins, or unrelated UI redesign.
- Never: expose arbitrary filesystem access to the frontend or commit secrets.

## Success Criteria

- The repository has a Tauri 2 project structure and no Dioxus dependencies.
- The frontend can load images, paginate, save settings, and download images
  through registered Tauri commands.
- Rust tests and formatting pass; the Vite production build succeeds.

# Spec: Dreamland Tauri migration

## Objective

Replace the Dioxus desktop shell with Tauri 2 while preserving Dreamland's
gallery, API pagination, settings, and image-download behavior.

## Commands

- Check Rust: `cargo check --manifest-path src-tauri/Cargo.toml`
- Test Rust: `cargo test --manifest-path src-tauri/Cargo.toml`
- Format Rust: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- Check frontend: `bun run build`
- Run desktop app: `bun run tauri:dev`

## Project Structure

- `index.html` — Vite entry document
- `src/` — React frontend
- `dist/` — Vite build output loaded by Tauri
- `src-tauri/` — Tauri application and Rust commands
- `tasks/` — migration plan and acceptance checklist

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
- Ask first: new native capabilities, plugins, or unrelated UI redesign.
- Never: expose arbitrary filesystem access to the frontend or commit secrets.

## Success Criteria

- The repository has a Tauri 2 project structure and no Dioxus dependencies.
- The frontend can load images, paginate, save settings, and download images
  through registered Tauri commands.
- Rust tests and formatting pass; the Vite production build succeeds.

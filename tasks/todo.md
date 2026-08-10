- [x] Add the Tauri application shell and Rust command boundary.
- [x] Port the gallery UI to React/TypeScript/Vite.
- [x] Establish the Cargo workspace and runtime/provider crate boundaries.
- [x] Add Tailwind CSS and TanStack Query frontend foundations.
- [x] Remove the Dioxus shell and update project documentation.
- [x] Run frontend, Rust formatting, and Tauri checks.

The remaining follow-up work is API approval, runtime configuration, and
product capability work.

The product roadmap is documented in [docs/ROADMAP.md](../docs/ROADMAP.md).

- [ ] Approve [API and runtime design v1](../docs/API-V1.md) before code.
- [x] Add the Nix project environment for macOS.
- [x] Add uv scripts for daily tooling and platform-aware checks.
- [ ] Adopt TOML runtime settings with a legacy JSON fallback.
- [ ] Define application-data storage for favorites and tags.
- [ ] Implement the provider contract and registry.
- [ ] Add provider-aware search, tags, favorites, and batch downloads.

# Architecture decisions

ADRs are the canonical record for accepted decisions. This page is only the
current index; it deliberately does not duplicate their full rationale.

## Accepted

| ADR | Decision |
| --- | --- |
| [0001](adr/0001-platform-and-toolchain.md) | macOS/Windows first-class; Nix project toolchain with uv daily scripts |
| [0002](adr/0002-frontend-stack.md) | React, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query |
| [0003](adr/0003-rust-workspace-boundaries.md) | Cargo workspace with isolated core, runtime, site adapter, and Tauri shell crates |
| [0004](adr/0004-site-runtime-boundary.md) | Rust-owned site/runtime boundary; `yandere` is one site ID |
| [0005](adr/0005-toml-runtime-configuration.md) | TOML runtime settings with a legacy JSON fallback |

## Pending gates

- [API and runtime design v1](API-V1.md) must be approved before site
  capabilities or registry behavior are implemented.
- Search, favorites, local tags, batch downloads, and the final site set
  remain roadmap decisions, not implementation commitments.
- Release packaging, signing, distribution, and update strategy have no plan
  yet.
- [ADR 0006](adr/0006-roadmap-and-github-issues.md) proposes GitHub Issues as
  an execution mirror for approved roadmap work; no remote issues have been
  created yet.

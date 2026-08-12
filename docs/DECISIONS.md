# Architecture decisions

Last updated: 2026-08-12.

ADRs are the canonical record for accepted decisions. This page is only the
current index; it deliberately does not duplicate their full rationale.

## Accepted

| ADR | Decision |
| --- | --- |
| [0001](adr/0001-platform-and-toolchain.md) | macOS/Windows first-class; Nix project toolchain with uv daily scripts |
| [0002](adr/0002-frontend-stack.md) | React, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query |
| [0003](adr/0003-rust-workspace-boundaries.md) | Cargo workspace with isolated core, shared protocol, runtime, site adapter, and Tauri shell crates |
| [0004](adr/0004-site-runtime-boundary.md) | Rust-owned site/runtime boundary; `yandere` is one site ID |
| [0005](adr/0005-toml-runtime-configuration.md) | TOML runtime settings with per-site sections |
| [0007](adr/0007-api-v1-site-contract.md) | Object-safe site capability ports and registry composition |
| [0008](adr/0008-frontend-visual-language.md) | Material 3 Expressive content system with restrained Liquid Glass shell and opaque cross-platform fallback |
| [0010](adr/0010-solid-layer-boundaries.md) | SOLID audit baseline for core, registry, runtime, and concrete site layers |

## Proposed, not yet accepted

- [ADR 0009](adr/0009-runtime-operation-context-and-common-services.md)
  defines an explicit lifecycle context plus typed runtime services for auth,
  cache, logging, configuration, and persistence. It deliberately does not
  add a generic Go-style dependency bag or claim that the current switchboard
  is already a trait registry.

## Pending gates

- [API and runtime design v1](API-V1.md) is the implementation baseline for
  the object-safe capability ports and registry. Each new site still needs its
  own capability and live-acceptance gate.
- The Yande release ([YANDE-RELEASE-GATE.md](YANDE-RELEASE-GATE.md), Y-01
  through Y-09) and the "Proposed 0.1.0 gate"
  ([ROADMAP.md](ROADMAP.md)) are the same milestone: Yande-scoped search,
  favorites, and pool/ZIP ship as part of v0.1.0, not deferred to Phase 2/3.
  [USER-STORIES-V1.md](USER-STORIES-V1.md)'s "v1" is that same milestone, not
  a separate later profile. Phase 2/3 remain for generalizing search and
  favorites to a second site and for local tags/notes/batch workflows, which
  are still roadmap decisions, not implementation commitments.
- Release packaging, signing, distribution, and update strategy have no plan
  yet.
- [ADR 0006](adr/0006-roadmap-and-github-issues.md) proposes GitHub Issues as
  an execution mirror for approved roadmap work; no remote issues have been
  created yet.

# Architecture decisions

Last updated: 2026-08-11.

ADRs are the canonical record for accepted decisions. This page is only the
current index; it deliberately does not duplicate their full rationale.

## Accepted

| ADR | Decision |
| --- | --- |
| [0001](adr/0001-platform-and-toolchain.md) | macOS/Windows first-class; Nix project toolchain with uv daily scripts |
| [0002](adr/0002-frontend-stack.md) | React, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query |
| [0003](adr/0003-rust-workspace-boundaries.md) | Cargo workspace with isolated core, runtime, site adapter, and Tauri shell crates |
| [0004](adr/0004-site-runtime-boundary.md) | Rust-owned site/runtime boundary; `yandere` is one site ID |
| [0005](adr/0005-toml-runtime-configuration.md) | TOML runtime settings with a legacy JSON fallback (accepted; implementation pending -- `AppConfig` still reads/writes JSON only) |
| [0008](adr/0008-frontend-visual-language.md) | Material 3 Expressive content system with restrained Liquid Glass shell and opaque cross-platform fallback |

## Proposed, not yet accepted

- [ADR 0007](adr/0007-api-v1-site-contract.md) proposes the v1 site-capability
  trait set: `PostQueryCapability`, `TagSuggestionCapability`,
  `PostLookupCapability`, `RemoteFavoriteCapability`, `SiteAuth`,
  `RemoteFavoriteListCapability`, `RemoteCollectionCapability`, and
  `CollectionDownloadCapability`. This is now reconciled with
  [ARCHITECTURE-V1.md](ARCHITECTURE-V1.md)'s feature-to-flow bindings (one
  list, not two). The ADR's own status is still "proposed research
  hypothesis," pending the site-wide matrix review.

## Pending gates

- [API and runtime design v1](API-V1.md) and [ADR 0007](adr/0007-api-v1-site-contract.md)
  must be approved before site capabilities or registry behavior are
  implemented.
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

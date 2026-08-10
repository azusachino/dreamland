# Dreamland roadmap

This is a product roadmap, not a release schedule. Release packaging,
signing, distribution, and store submission are intentionally outside the
current plan.

The roadmap remains canonical in this repository. GitHub Issues are an
execution mirror for approved work: each issue should be actionable, link to
the relevant roadmap/ADR/API section, and avoid introducing a decision that
is not documented here.

The long-term goal is to exceed MoeLoaderP’s provider breadth and workflow
depth with a cleaner, provider-neutral runtime. The roadmap reaches that goal
in layers; v0.1.0 is the contract and parity foundation, not the final feature
set.

## Guiding constraints

- macOS and Windows are the first-class desktop platforms; Linux is not a
  planned target.
- React owns presentation and interaction state.
- Rust owns the runtime: provider requests, configuration, persistence, and
  downloads. The frontend does not gain direct network or filesystem access.
- Providers are a set, not a single hardcoded API. `yande.re` is one provider
  in that set and must remain an adapter behind the provider boundary.
- TOML is for human-authored runtime configuration. Favorites, tags, cache
  metadata, and download state are application data and must not be forced into
  the config file.

## Phase 0 — fresh desktop rework

This is the current Tauri/React foundation:

- Tauri 2 desktop shell;
- React 19 + TypeScript + Vite + Bun frontend;
- Tailwind CSS styling and TanStack Query IPC state;
- Rust command boundary for API calls, settings, and downloads;
- Nix development environment for macOS;
- desktop-only macOS/Windows icon set.

Reference rationale: [REFERENCE-LEARNINGS.md](REFERENCE-LEARNINGS.md).

## Proposed 0.1.0 gate

This is a development milestone proposal, not a release commitment. Before
calling the foundation complete, Dreamland should have:

- an approved and implemented API/runtime design v1;
- a provider registry and one complete `yande.re` adapter behind it;
- provider-neutral browse, pagination, settings, validation, and download
  commands;
- TOML runtime configuration with the legacy JSON fallback;
- Rust tests for provider normalization, command serialization, configuration,
  and safe downloads;
- passing daily checks on macOS and a native Windows validation run.

Search, favorites, local tags, and batch downloads remain subsequent roadmap
phases. They are not prerequisites for this foundation milestone, and no
release packaging is implied by the `0.1.0` label.

## Phase 1 — provider and runtime foundation

Blocked until [API and runtime design v1](API-V1.md) is approved. Then preserve
and formalize the existing API/runtime behavior before adding product features:

- implement the approved provider contract and registry;
- keep `yande.re` as one complete provider adapter;
- represent provider capabilities explicitly, especially pagination and
  search, instead of assuming every provider behaves like yande.re;
- move user configuration from JSON to TOML with a legacy JSON fallback;
- keep provider requests, validation, persistence, and downloads in Rust;
- expose provider-neutral commands to React.

## Phase 2 — discovery

Add the first user-facing product expansion:

- provider-aware search;
- tag browsing and tag-based filtering;
- clear loading, empty, provider-error, and unavailable-capability states;
- preserve pagination semantics per provider.

Search and tags must follow provider capabilities. The UI should not present a
search control for a provider that cannot implement it.

## Phase 3 — personal organization

- favorite posts across providers;
- view and filter favorites;
- local tags/notes attached to favorites where the product model supports it;
- stable provider/post identifiers so favorites survive provider refreshes.

Before implementation, choose the storage model for application data. TOML is
not the default for mutable favorites and tags; the options need to be weighed
against concurrency, querying, backup, and portability requirements.

## Phase 4 — batch workflow

- select multiple posts;
- queue batch downloads;
- show progress and per-item failures;
- support retry and cancellation;
- avoid duplicate downloads and preserve safe provider-aware filenames.

## Explicitly not planned yet

- release packaging or distribution;
- signing and notarization;
- automatic updates;
- Linux support;
- mobile applications.

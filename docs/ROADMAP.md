# Dreamland roadmap

Last updated: 2026-08-11.

This is a product roadmap, not a release schedule. Release packaging,
signing, distribution, and store submission are intentionally outside the
current plan.

The roadmap remains canonical in this repository. GitHub Issues are an
execution mirror for approved work: each issue should be actionable, link to
the relevant roadmap/ADR/API section, and avoid introducing a decision that
is not documented here.

The long-term goal is to exceed MoeLoaderP’s site breadth and workflow
depth with a cleaner, site-neutral runtime. The roadmap reaches that goal
in layers; v0.1.0 is the contract and parity foundation, not the final feature
set.

## Guiding constraints

- macOS and Windows are the first-class desktop platforms; Linux is not a
  planned target.
- React owns presentation and interaction state.
- Rust owns the runtime: site requests, configuration, persistence, and
  downloads. The frontend does not gain direct network or filesystem access.
- Sites are a set, not a single hardcoded API. `yande.re` is one site in that
  set and must remain an adapter behind the site boundary.
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

This is the same milestone as [YANDE-RELEASE-GATE.md](YANDE-RELEASE-GATE.md)'s
"the Yande release" -- not a narrower technical-only foundation that precedes
it. Before calling v0.1.0 complete, Dreamland should have:

- an approved and implemented API/runtime design v1;
- a site registry and one complete `yande.re` adapter behind it;
- site-neutral browse, pagination, settings, validation, and download
  commands;
- Yande tag search and tag suggestions (Y-02/Y-03);
- Yande day/week/month popular browsing (Y-04);
- browser-session login and remote favorite add/remove/list (Y-05/Y-06/Y-07);
- Yande pool browse and pool ZIP download (Y-09);
- TOML runtime configuration with explicit per-site sections;
- Rust tests for site normalization, command serialization, configuration,
  and safe downloads;
- passing daily checks on macOS and a native Windows validation run;
- every G-01 through G-10 gate in YANDE-RELEASE-GATE.md at `PASS` or an
  explicitly approved non-live exception.

Local tags/notes, multi-site search and favorites generalization, and batch
downloads remain subsequent roadmap phases. They are not prerequisites for
this milestone, and no release packaging is implied by the `0.1.0` label.

## Phase 1 — site and runtime foundation

Blocked until [API and runtime design v1](API-V1.md) is approved. This phase
delivers the full Yande release scope above (Y-01 through Y-09), not just
registry plumbing:

- implement the approved site contract and registry;
- implement `yande.re` as one complete site adapter, including tag search,
  popular browsing, auth, remote favorites, and pool/ZIP;
- represent site capabilities explicitly, especially pagination and search,
  instead of assuming every site behaves like yande.re;
- keep user configuration in TOML with explicit per-site sections;
- keep site requests, validation, persistence, and downloads in Rust;
- expose site-neutral commands to React.

## Phase 2 — discovery, generalized

Yande's own tag search and popular browsing ship in v0.1.0 (US-Y-03/04, part
of Phase 1 above). This phase is about generalizing that to a second site,
not introducing search for the first time:

- a second site's search/tag capabilities, exposed through the same
  site-neutral commands;
- clear loading, empty, site-error, and unavailable-capability states across
  more than one site;
- preserve pagination semantics per site, including Yande's selected-window
  popular queries and sites that expose different popular-feed contracts.

Search and tags must follow site capabilities. The UI should not present a
search control for a site that cannot implement it.

## Phase 3 — personal organization, generalized

Yande's own remote favorites ship in v0.1.0 (US-Y-09/10, part of Phase 1
above). This phase is about generalizing favorites across sites and adding
local organization on top, not introducing favorites for the first time:

- favorite posts across more than one site, with stable per-site identifiers
  so favorites survive site refreshes;
- view and filter favorites across sites;
- local tags/notes attached to favorites where the product model supports it.

Before implementation, choose the storage model for application data. TOML is
not the default for mutable favorites and tags; the options need to be weighed
against concurrency, querying, backup, and portability requirements.

## Phase 4 — batch workflow

- select multiple posts;
- queue batch downloads;
- show progress and per-item failures;
- support retry and cancellation;
- avoid duplicate downloads and preserve safe site-aware filenames.

## Explicitly not planned yet

- release packaging or distribution;
- signing and notarization;
- automatic updates;
- Linux support;
- mobile applications.

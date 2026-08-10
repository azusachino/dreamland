# ADR 0004: Site and runtime boundary

- Status: Accepted
- Date: 2026-08-10

## Context

Dreamland will support a set of image-board sites. `yande.re` is an
important site, but it must not become the application-wide API contract.
The reference projects also show that remote metadata and local favorites,
tags, cache, and download state have different ownership.

## Decision

Rust owns site HTTP, response decoding, validation, configuration,
application data, and downloads. React communicates through narrow,
site-neutral Tauri commands and never receives arbitrary filesystem or
network capabilities.

The stable site ID for the Yande.re adapter is `yandere`. Site capabilities
belong at the site boundary. Local favorites, tags, cache, and download
records belong to Dreamland-owned runtime stores rather than site DTOs.

The current command DTOs are provisional. API v1 approval is required before
site capabilities, the registry, or the final site-neutral command surface
are implemented.

## Consequences

- A site adapter can change its remote response shape without changing
  the frontend contract.
- The frontend cannot bypass runtime validation or persistence rules.
- Yandere is the first adapter, not the application’s universal site.

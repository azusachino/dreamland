# ADR 0004: Provider and runtime boundary

- Status: Accepted
- Date: 2026-08-10

## Context

Dreamland will support a set of image-board providers. `yande.re` is an
important provider, but it must not become the application-wide API contract.
The reference projects also show that remote metadata and local favorites,
tags, cache, and download state have different ownership.

## Decision

Rust owns provider HTTP, response decoding, validation, configuration,
application data, and downloads. React communicates through narrow,
provider-neutral Tauri commands and never receives arbitrary filesystem or
network capabilities.

The stable provider ID for the Yande.re adapter is `yandere`. Provider
capabilities belong at the provider boundary. Local favorites, tags, cache,
and download records belong to Dreamland-owned runtime stores rather than
provider DTOs.

The current command DTOs are provisional. API v1 approval is required before
provider traits, the registry, or the final provider-neutral command surface
are implemented.

## Consequences

- A provider adapter can change its remote response shape without changing
  the frontend contract.
- The frontend cannot bypass runtime validation or persistence rules.
- Yandere is the first adapter, not the application’s universal API.

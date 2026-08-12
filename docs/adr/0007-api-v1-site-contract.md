# ADR 0007: API v1 site contract

- Status: Accepted; initial ports implemented in 0.1.1
- Date: 2026-08-12

## Context

Dreamland’s first API draft separated browse, free-text search, tag search,
and post lookup into several site capabilities, while leaving their real
semantics open. Yande’s API and the Yande adapter in MoeLoaderP show that
browse and tag search are one post-list operation, tag suggestions are a
separate endpoint, and the site returns several URL variants and rich
metadata. MoeLoaderP also exposes client features that its Yande adapter does
not actually implement, including date search, ordering, multi-keyword modes,
and cursor pagination.

The Rust trait-object boundary must preserve optional capabilities without
forcing sites to implement fake methods. It must also keep saved-query
definitions, caching, and download queue state separate from remote site
models. A remote favorite is not a local bookmark.

## Decision

The current proposal is one required post-query capability and several
optional capabilities. This list is reconciled with
[ARCHITECTURE-V1.md](../ARCHITECTURE-V1.md)'s feature-to-flow bindings, which
already required the last three entries for Y-07/Y-09 before this ADR listed
them:

- `PostQueryCapability` handles explicit browse, site query-expression
  search, and Yande’s day/week/month popular modes. Yande popular modes are
  ordinary `/post.json` page queries with a date expression and
  `order:score`, so page/limit continuation stays within the selected window.
- `TagSuggestionCapability` is optional and returns tag names plus optional
  site metadata.
- `PostLookupCapability` is optional and hydrates one `PostRef`.
- `RemoteFavoriteCapability` is optional and sets remote favorite state for a
  site-scoped post reference.
- `SiteAuth` is optional; Yande uses a browser session and its `user_info`
  cookie (with legacy `user_id` compatibility) rather than a generic
  username/password request.
- `RemoteFavoriteListCapability` is optional and reads the authenticated
  current user's remote favorites, distinct from setting favorite state.
- `RemoteCollectionCapability` is optional and exposes pool metadata and
  ordered pool posts.
- `CollectionDownloadCapability` is optional and resolves a pool's archive
  (ZIP) download target.

The base `SiteAdapter` owns identity and exposes the required and optional
object-safe capability ports. `SiteRegistry` validates that descriptor
capability flags agree with those accessors. Current remote calls receive the
validated network policy; runtime session cancellation and operation IDs stay
above the adapter port until the operation-context hook is wired through.
The runtime owns stale-result suppression, local filtering, downloads, and
stable application errors.

Normalized posts use `(site_id, post_id)` identity, optional metadata, an
explicit content-rating mapping, algorithm-labelled checksums, and stable
download variant kinds. Yande’s direct preview/sample/jpeg/original URLs are
the v1 download model. Frontend commands accept references and variant kinds,
never site JSON, URLs, or filesystem paths.

Cursor pagination, tag mutation, and site-specific download-resolution
extensions remain deferred until a concrete site and product flow require
them. Yande’s authenticated favorite mutation is part of the first site
profile. New capabilities must be added as separate ports and registered
through `SiteRegistry`; they must not become shared optional methods with
fake implementations.

## Potential consequences

- Yande can be implemented without pretending that its tag grammar is
  portable full-text search.
- The first site remains complete enough for browse, safe/explicit
  filtering, tag suggestions, lookup, direct downloads, the remote favorite
  list, and pool browse/ZIP.
- A future cursor-based or authenticated site requires a deliberate
  contract extension instead of hidden state or fake optional methods.
- Local product features can evolve without coupling storage to site DTOs.

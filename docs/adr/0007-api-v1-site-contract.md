# ADR 0007: API v1 site contract

- Status: Proposed research hypothesis
- Date: 2026-08-10

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

## Proposal under review

The current proposal is one required post-query capability and several
optional capabilities:

- `PostQueryCapability` handles explicit browse, site query-expression
  search, and Yande’s day/week/month popular modes. Page/limit pagination is
  used only where the selected mode supports it.
- `TagSuggestionCapability` is optional and returns tag names plus optional
  site metadata.
- `PostLookupCapability` is optional and hydrates one `PostRef`.
- `RemoteFavoriteCapability` is optional and sets remote favorite state for a
  site-scoped post reference.
- `SiteAuth` is optional; Yande uses a browser session and `user_id`
  cookie rather than a generic username/password request.

The base `SiteAdapter` owns identity and exposes the required and optional
capability trait objects. The registry validates that descriptor capability
flags agree with those accessors. All remote calls receive cancellation and a
runtime operation ID. The runtime owns stale-result suppression, local
filtering, downloads, and stable application errors.

Normalized posts use `(site_id, post_id)` identity, optional metadata, an
explicit content-rating mapping, algorithm-labelled checksums, and stable
download variant kinds. Yande’s direct preview/sample/jpeg/original URLs are
the v1 download model. Frontend commands accept references and variant kinds,
never site JSON, URLs, or filesystem paths.

Cursor pagination, tag mutation, and site-specific download-resolution
traits remain deferred until a concrete site and product flow require
them. Yande’s authenticated favorite mutation is now part of the first
site profile, but this ADR remains a proposed hypothesis pending the
site-wide matrix review.

## Potential consequences

- Yande can be implemented without pretending that its tag grammar is
  portable full-text search.
- The first site remains complete enough for browse, safe/explicit
  filtering, tag suggestions, lookup, and direct downloads.
- A future cursor-based or authenticated site requires a deliberate
  contract extension instead of hidden state or fake optional methods.
- Local product features can evolve without coupling storage to site DTOs.

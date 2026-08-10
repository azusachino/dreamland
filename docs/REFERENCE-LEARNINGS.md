# Reference learnings

These notes capture what Dreamland should learn from the read-only references,
not what it should copy wholesale.

## `xiao-po/yande`

- Separate remote HTTP sources from local persistence. Its application has an
  HTTP data source pool and a DAO data source instead of treating favorites as
  provider data.
- Persist favorites, tag shortcuts, blocked tags, and download status/path.
- Keep rating filtering and blocked-tag filtering in the runtime/service layer,
  before results reach the UI.
- A provider can expose both post queries and tag autocomplete; these are
  distinct operations.
- A large provider response model is useful as an adapter DTO, but should not
  become Dreamland’s cross-provider domain model.

## `xplusky/MoeLoaderP`

- `SiteManager` and `MoeSite` demonstrate a provider registry with per-site
  capability flags and per-provider settings.
- `SearchSession` demonstrates why search needs ownership of query state,
  page progression, history, cancellation, and stale-result handling.
- `MoeItem`/`UrlInfo` demonstrate that a post may have children and multiple
  download variants, including referers, URL resolution, file size, and
  post-download processing.
- `MoeDownloader` demonstrates bounded concurrency, progress, retries, and
  batch-oriented download state.
- Custom-site definitions show the eventual breadth target, but a declarative
  HTML scraper is too much surface for v0.1.0.

## Dreamland decisions

- Adopt a small provider core plus optional capability traits; avoid one giant
  provider interface with fake unsupported methods.
- Use `(provider_id, post_id)` as the stable local identity.
- Keep remote metadata, local favorites/tags, cache records, and download
  records in separate runtime-owned stores.
- Treat yande.re as one complete provider adapter, not the universal API.
- Build toward MoeLoaderP-level breadth in phases, with the API v1 design as
  the gate before implementing provider or runtime traits.

# Reference learnings

These notes capture what Dreamland should learn from the pinned research
sources,
not what it should copy wholesale.

## Vendored `xplusky/MoeLoaderP`

Dreamland vendored MoeLoaderP as a nested git submodule at
`vendor/moeloaderp` during research, pinned to commit
`0025dd999306258103ed2b239b2132d3913c99e0`. That submodule has since been
dropped; the pinned commit remains the source of truth for the site inventory
below, and [`xplusky/MoeLoaderP`](https://github.com/xplusky/MoeLoaderP) at
that commit is where to read it.

- `SiteManager` and `MoeSite` demonstrate a site registry with per-site
  capability flags and per-site settings.
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

- Adopt a small site core plus optional capability traits; avoid one giant
  site interface with fake unsupported methods.
- Use `(site_id, post_id)` as the stable local identity.
- The pinned MoeBooru UX reference stores saved query definitions and pin/order
  state. Dreamland v1 follows that observed model for local state: saved
  queries, query history, cache, and download records. Yande favorites remain
  remote site state; a local post-bookmark collection is a deferred,
  explicitly separate feature.
- Treat yande.re as one complete site adapter, not the universal API.
- Build toward MoeLoaderP-level breadth in phases, with the API v1 design as
  the gate before implementing site/runtime capabilities.

## Yande and MoeLoaderP evidence used for API v1

The vendored MoeLoaderP checkout implements Yande as a `BooruSite`:

- `YandeSite` uses one paged post query, passing `page`, `limit`, and a keyword
  expression as `tags`; safe mode appends `rating:s`.
- The same adapter exposes tag autocomplete through `tag.xml`, ordered by
  count, with a bounded result count.
- The shared booru parser normalizes tags, ID, dimensions, posting account, source,
  rating, date, score, preview URL, sample URL, JPEG URL, original URL, and
  original file size.
- The shared site configuration enables keyword, rating, resolution, and
  score-aware behavior, but resolution/orientation and safe/explicit filtering
  happen locally after the response is decoded.
- Yande enables cookie-based account detection. Its current web client uses
  the post-vote endpoint for remote favorite add/remove, so account/session
  state and a site-specific favorite action are part of the Yande v1
  profile even though they are optional across the wider site API.

The adapter’s generic `SearchPara` contains date, ordering, multi-keyword, and
cursor fields, but `YandeSite` does not translate those fields into its query.
Those controls are evidence about the larger product space, not evidence that
Yande supports each one in Dreamland v1.

The live JSON API also confirms that Yande supports both legacy array responses
and the `api_version=2` `{ "posts": [...] }` envelope for `/post.json`, direct
ID filtering through `id`, tag filtering through `tags`, page/limit controls,
and `/tag.json` results with `name`, `count`, `type`, and `ambiguous` fields.

The live API also exposes `/post/popular_by_day.json`,
`/post/popular_by_week.json`, and `/post/popular_by_month.json`. At the current
runtime watermark each returned a fixed 40-item window; `page` and `limit` did
not change the result window. The Yande web client submits favorite changes to
`/post/vote.json` with score 3 for favorite and score 2 to remove it, behind a
logged-in browser session. Public pool pages expose ordered pool posts and link
to `/pool/zip/:id`; the observed anonymous ZIP request redirects to login.
Yande also resolves an existing image conservatively by checksum through a
query such as `md5:<hash>`, returning one matching post in the observed case.

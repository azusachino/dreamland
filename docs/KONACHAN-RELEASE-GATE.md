# Konachan adapter release gate

Status: safe browse, public pool, safe post lookup, and browser post-route integration implemented, 2026-08-11. This is not a claim
that Cloudflare bot protection has been solved.

## Implemented

| Gate | Evidence | Status |
| --- | --- | --- |
| K-01 | `dreamland-site-konachan` descriptor and active registry entry | Pass |
| K-02 | Safe API config uses `https://konachan.net/post.json`; browser recovery URL remains `https://konachan.com/post` | Pass |
| K-03 | Real response shape fixture maps numeric Unix `created_at`, post metadata, ratings, and media variants into `Post` | Pass |
| K-04 | Page-numbered search, safe-policy rejection, and tag-suggestion routing are covered by adapter/site-registry tests | Pass |
| K-05 | Direct `.com` API request was observed returning Cloudflare `403`; the adapter reports bot-protection guidance instead of retrying it as a generic server error | Observed limitation |
| K-06 | The app exposes `konachan` in the site selector and provides an explicit `open site` action for the site-owned `https://konachan.com/post` browser route; feed errors offer the same recovery | Pass; static/runtime command coverage, live visual acceptance pending |
| K-07 | Public pool metadata uses `/pool.json`; ordered safe-visible posts use `/pool/show.json?id=…`; the app exposes browsing but not pool ZIP downloads | Pass; live response shape and adapter fixtures verified |
| K-08 | Selecting a Konachan result hydrates the exact post through the safe API's `id:<post-id>` query, rejects non-numeric IDs, and keeps the feed snapshot visible on lookup failure | Pass; live `id:407162` response and adapter/runtime tests verified |
| K-09 | Pool browsing accepts a title query and resets page-number pagination for a new search; the UI exposes `search pools` and `clear` | Pass; live `Kona_Garage` query returned one match on page 1 and none on page 2; shared transport test and typecheck pass |
| K-10 | Post detail exposes a site-owned `open <site> post` action that validates numeric IDs and opens `/post/show/<id>` in a dedicated browser window | Pass; adapter and registry URL tests pass; `.com` HTML access remains subject to bot protection |
| K-11 | Post detail author identity is an exact `user:<name>` search action, kept distinct from artwork tag chips | Pass; live Konachan `otaku_emmy` and Yande `moonian` queries returned matching posts; TypeScript and full gate pass |
| K-12 | Post detail parent/child state exposes exact `id:<parent-id>` and `parent:<post-id>` searches, returning to the result view | Pass; live Konachan `id:407153` and `parent:407153` queries returned the observed parent/child relationship; TypeScript and full gate pass |
| K-13 | Post detail can explicitly load related tags from the site endpoint and use them as safe-policy searches | Pass; live `tag/related.json?tags=cirno` returned related tuples; decoder normalizes string/numeric counts and focused tests pass |

## Not advertised yet

- explicit/questionable content through the `.com` host;
- browser-session auth and remote favorites;
- pool ZIP archives;
- authorized live acceptance from a clean browser profile.

The official Konachan API documentation describes `/post.json` with `tags`,
`page`, and `limit` parameters. The safe `.net` API is therefore a valid
initial browse/search transport, but `.com` bot protection remains a release
boundary rather than something the runtime should bypass.

Related-tag names and counts are site metadata and are not treated as
rating-filtered post results. Dreamland loads them only after an explicit
detail action; clicking a related tag returns to the ordinary content-policy
filtered post search.

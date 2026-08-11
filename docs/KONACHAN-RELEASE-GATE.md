# Konachan adapter release gate

Status: adapter slice implemented, 2026-08-11. This is not a claim that the
Konachan site is fully integrated into the default Yande UI or that Cloudflare
bot protection has been solved.

## Implemented

| Gate | Evidence | Status |
| --- | --- | --- |
| K-01 | `dreamland-site-konachan` descriptor and active registry entry | Pass |
| K-02 | Safe API config uses `https://konachan.net/post.json`; browser recovery URL remains `https://konachan.com/post` | Pass |
| K-03 | Real response shape fixture maps numeric Unix `created_at`, post metadata, ratings, and media variants into `Post` | Pass |
| K-04 | Page-numbered search, safe-policy rejection, and tag-suggestion routing are covered by adapter/site-registry tests | Pass |
| K-05 | Direct `.com` API request was observed returning Cloudflare `403`; the adapter reports bot-protection guidance instead of retrying it as a generic server error | Observed limitation |

## Not advertised yet

- explicit/questionable content through the `.com` host;
- browser-session auth and remote favorites;
- pools and pool ZIP archives;
- Konachan site selection in the default Yande-first frontend; and
- authorized live acceptance from a clean browser profile.

The official Konachan API documentation describes `/post.json` with `tags`,
`page`, and `limit` parameters. The safe `.net` API is therefore a valid
initial browse/search transport, but `.com` bot protection remains a release
boundary rather than something the runtime should bypass.

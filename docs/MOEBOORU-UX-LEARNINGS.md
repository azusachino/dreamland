# MoeBooru UX learnings

Status: research notes, 2026-08-10. These observations come from a nested
read-only UX reference checkout of [`yueeng/moebooru`](https://github.com/yueeng/moebooru)
(vendored at `vendor/moebooru` during research, since dropped as a submodule),
pinned to:

```text
5bcf76644de594e136865eb70d486b916152de4b
```

The reference app supports Yande and Konachan. It is useful evidence for
interaction shape, not a specification that Dreamland must copy.

## Product shape

- The default home is a post grid. The initial tab is a popular feed.
- The top-level content is tabbed: the popular feed plus saved query tabs.
- A floating action button expands into a navigation menu for popular periods
  and related views.
- Site selection is effectively a build/host choice in the reference app;
  Dreamland can make it an explicit selector while keeping `yandere` as the
  default site.

## Query discovery

The query editor is more than a plain search box:

- the keyword field offers remote tag suggestions after one character;
- suggestions show tag type, tag name, and alias information;
- multiple terms are tokenized by spaces;
- a leading `-` preserves negative-tag intent;
- an advanced query sheet exposes site controls such as rating, order,
  vote, date, width, height, megapixels, ID, user, score, MD5, source, parent,
  and pool;
- a query can be saved with a friendly name and pinned for quick access.

This makes tag autocomplete a core exploration capability. The user should not
need to know Yande's exact tag grammar before they can discover useful posts.
The stable API should expose suggestions and preserve the complete site
expression, while the UI can begin with a simple keyword field and progressively
reveal advanced controls.

### Author and tag identity

MoeBooru receives `author` and `creator_id` as post fields. It uses them for
the posting account/user route (`user:<name>` and the account/profile screen).
That is different from artist tags: artist tags are separate typed tags and
are the normal way to search an artist's artworks. Dreamland should therefore
show the complete tag list in post details and keep the posting account as
separate metadata; neither should silently become the canonical filename.

## Popular and pool exploration

The reference app offers popular day, week, month, and year views. It also
allows date navigation with a date picker, and its query model includes pool
selection. Therefore the Dreamland UX stories should distinguish:

- a popular period plus a selected date/window; and
- a pool/gallery sequence with ordered child posts.

Yande adds a third pool action: the pool page exposes a site ZIP download.
That is a collection-archive operation, not batch downloading the currently
selected cards one by one. It should be shown on a pool page and use the
site's auth/capability state.

The vendored implementation resolves the popular-feed shape: `PopularActivity`
selects a day/week/month date window, while its `ImageDataSource` pages the
ordinary `post.json` query with the selected `date` expression,
`order:score`, and a requested load size. Dreamland follows that behavior.
Yande's separate `popular_by_*` endpoints remain fixed windows and are not the
scrollable feed path.

## Post interaction and download

- Tapping a card opens a preview/detail screen with swipe navigation.
- Long-pressing a card exposes tag inspection, local download, and favorite.
- A post can expose preview, sample/JPEG, and original media choices; Dreamland's detail view now starts with the sample image and falls back safely.
- The reference app's quality preference chooses between JPEG and original;
  its default is not automatically “best quality”. Dreamland may choose best
  quality as its product default, but that is a deliberate UX decision.
- Downloads show progress, duplicate/existing-file handling, notifications,
  and a configurable download location.

Batch download is not clearly established as a required reference workflow;
it should remain a separate user story rather than silently becoming part of
the v1 API.

## Account and favorite behavior

- Login is account state detected through the site session cookie.
- Favorite is a remote authenticated post action, separate from saved query
  definitions.
- The reference user screen exposes “Favorites” through a site query such
  as `vote:3:<username> order:vote`, alongside uploads through `user:<name>`.

This supports the current hypothesis that the Yande favorite page may be a
site-specific exact query, but it does not prove that the query is safe or
complete for private authenticated favorites. That still requires authorized
Yande verification.

## Local persistence lessons

The reference SQLite database stores saved query definitions and pin/order
state. It does not make “saved” mean downloaded posts. Dreamland should keep
these concepts separate:

1. saved/replayable queries;
2. download queue/history; and
3. remote account favorites.

Only the first two are inherently local Dreamland state. Remote favorite state
belongs to the site session and may be unavailable when logged out.

The local library should be searchable by the immutable tag snapshot captured
with a download. Date can remain a filter, but it is a poor primary organizer:
several unrelated posts commonly share the same day, while artist/character/
copyright tags are the user-facing discovery keys.

Downloads should be queued and staged through a local temporary cache. The
worker must abort with an existing-target result rather than overwrite a file;
arbitrary existing files are outside the queue's ownership boundary.

## UX implications for Dreamland

Before finalizing API v1, agree on these user stories:

1. Open Dreamland to Yandere's default post feed.
2. Switch site from a compact selector without losing the current query.
3. Browse popular periods and, if verified, historical windows.
4. Search with tag autocomplete, negative tags, and saved/pinned queries.
5. Open a post, inspect tags/media, and download the selected or default
   quality.
6. Download a selected post, observe progress, and open the local download
   location/history. Batch selection/execution remains a later decision.
7. Log in and view/add/remove remote favorites when the site supports it.
8. Open a pool, inspect its ordered posts, and request the site ZIP when
   available.
9. Configure site endpoint, download path, quality, cache, and logging.

Only after these stories are accepted should the API decide which concepts are
stable core, site capability, or deferred extension.

## Live Yande comparison

Read-only probes on 2026-08-10 found:

- `GET /tag.json?name=blu&limit=5&order=count` returns tag name, count, type,
  and ambiguity metadata suitable for autocomplete;
- `GET /post/popular_by_day.json?day=...&month=...&year=...` and the week/month
  equivalents return 40 posts, and changing the requested date changes the
  result window;
- ordinary `page`/`limit` parameters do not turn those popular responses into
  normal pagination;
- the vendored MoeBooru app instead uses `post.json` with
  `order:score` plus a date expression, and its normal page source provides
  infinite scroll within that selected window;
- `GET /pool.json` returns public pool records with ID, name, date, owner,
  visibility, and post count; `query` filters by title and `page`/`limit`
  paginate the result; `tags=pool:<id>` returns the pool's posts with normal
  page semantics;
- the public pool page links to `/pool/zip/<id>` for a ZIP archive; an
  unauthenticated request currently redirects to the login page;
- post responses expose preview, sample, JPEG, and original URLs with separate
  dimensions and file sizes, so “best quality” is a runtime selection policy,
  not a site field;
- the candidate favorite-page query `vote:3:<username> order:vote` is a
  plausible exact-tag mapping, but an anonymous probe cannot establish private
  visibility, current-user scoping, or authenticated ordering.

# MoeLoaderP site matrix

Status: research draft, 2026-08-10. This is the source inventory for API v1;
it is not an implementation approval.

## Source boundary

Dreamland vendored [`xplusky/MoeLoaderP`](https://github.com/xplusky/MoeLoaderP)
as a nested git submodule at `vendor/moeloaderp` during research (since
dropped as a submodule), pinned to:

```text
0025dd999306258103ed2b239b2132d3913c99e0
```

All observations in this document are from that checkout. The site list is the
`SiteManager.SetDefaultSiteList` registry, plus site classes that
exist in the source but are not registered by default.

## Registry strata

MoeLoaderP's short name is `yande`, but Dreamland's persisted site ID is
`yandere` and remains canonical for `PostRef`, config, and saved records. The
MoeLoaderP name is an adapter-source alias only.

The default registry contains these 15 sites:

| Site class | Short name | Family |
| --- | --- | --- |
| `PixivSite` | `pixiv` | authenticated API/client |
| `KonachanSite` | `konachan` | direct JSON |
| `KonachanNetSite` | `konachan.net` variant | direct JSON |
| `YandeSite` | `yande` (Dreamland canonical ID: `yandere`) | Booru |
| `GelbooruSite` | `gelbooru` | Booru/XML2 |
| `SankakuChanSite` | `sankakucomplex-chan` | authenticated cursor API |
| `DanbooruSite` | `danbooru` | Booru/JSON |
| `DeviantartSite` | `deviantart` | authenticated cursor API |
| `BehoimiSite` | `3dBooru` | Booru/XML |
| `SafebooruSite` | `safebooru` | Booru/XML |
| `LolibooruSite` | `lolibooru` | Booru/XML |
| `MiniTokyoSite` | `minitokyo` | authenticated HTML |
| `EshuuSite` | `e-shu` | HTML form/search |
| `ZeroChanSite` | `zerochan` | HTML |
| `WorldCosplaySite` | `worldcosplay` | JSON API |

X-mode adds `PixivR18Site`, `AtfbooruSite`, `Rule34Site`, and
`AnimePicturesSite`. The source also contains `BilibiliSite`, `KawaiinyanSite`,
`SankakuIdolSite`, and `YuriimgSite`; these classes are not in the default
registry at this commit. `CustomSite` is loaded from JSON and is a separate
declarative site path.

The registry distinction matters. “A site class exists” and “the product
offers this site” are different states, and API descriptors should expose
availability/configuration rather than silently promising every class.

## Capability families observed

| Family | Sites | Query and pagination | Result expansion | Auth/actions | Config evidence |
| --- | --- | --- | --- | --- | --- |
| Shared Booru adapter | Yande, Gelbooru, Danbooru, Behoimi, Safebooru, Lolibooru, Atfbooru, Rule34 | Page-numbered post query; XML, XML2, or JSON; generic tag hint path with site overrides | Preview/medium/large/original URLs; optional deferred detail (`Gelbooru`) | Cookie detection on several sites; no universal mutation action | Keyword/rating/resolution/score flags; max page size; detail URL/format overrides |
| Direct Booru-like adapters | Konachan and `Konachan.net` | Page-numbered JSON; multi-keyword handling | Preview/sample/original URLs | No common account action observed | Site query construction and tag hints are bespoke |
| Cursor/API clients | Pixiv, Sankaku Chan, DeviantArt | Opaque cursor continuation; categories and site-specific endpoints | Galleries/children; Pixiv deferred detail and ugoira media resolution | Pixiv/DeviantArt cookie sessions; Sankaku token/cookie session and online favorite | Categories, sort modes, R18 variant, account requirement, multi-keyword |
| Authenticated/scraped HTML | MiniTokyo, Eshuu, AnimePictures | Page-numbered or HTML-derived navigation; form redirects may be required | Preview/original or detail-page-derived URLs | Browser cookie/JWT detection; MiniTokyo login flow | Category menus, rating/resolution/score flags, site-specific login URL |
| HTML/JSON social/media | ZeroChan, WorldCosplay, Bilibili | Page-numbered requests; category/sort controls | Single posts can contain image groups; Bilibili can populate children | Bilibili has cookie detection but its online action is not implemented | Category menus and separate new/hot/search routes |
| Group/detail adapters | Yuriimg, Bilibili, Pixiv, CustomSite | Initial list followed by detail request | Child items, child counts, deferred detail, multiple variants | Varies by underlying site | Detail levels and child extraction are site-specific |
| Declarative custom site | `CustomSite` | URL templates, category APIs, first/follow-up page APIs | XPath/HTML extraction, detail levels, children, referers, URL transforms | Login URL plus cookie-name detection | JSON supplies endpoints, XPath rules, categories, pagination, auth key |

This table is a capability family map, not a claim that every site in a
row has identical semantics. In particular, Booru compatibility supplies a
normalization starting point, not a guarantee of identical tag grammar,
rating values, pagination limits, or authentication behavior.

## The MoeLoaderP internal contract

`MoeSite` has one required operation: `GetRealPageAsync(SearchPara,
CancellationToken)`. The same base class also provides optional autocomplete,
proxy selection, cookie verification, login-page metadata, logout, thumbnail,
and favorite hooks. `MoeSiteConfig` is a collection of UI/product hints:

- keyword, rating, resolution, score, multi-keyword, date-picker, account,
  thumbnail-button, favorite-button, and image-order flags;
- R18/custom-site markers;
- image-order definitions and a maximum page size;
- category trees whose child configuration is cloned from the parent.

`SearchPara` combines site intent and client filtering: keyword and
multi-keywords, page number or cursor, page size, rating visibility, local
resolution/orientation/file filters, sort/date, category indices, config, and
mirror selection. `SearchedPage` can carry page numbers and opaque cursors at
the same time, plus optional totals and a cloned next request.

`MoeItem` is broader than a single image row. It carries site identity,
numeric/string IDs, title/date/posting-account/source/description/tags/rating-like
state, dimensions, detail URL, child items/count, and deferred detail loading.
`UrlInfo` carries a media kind, URL, referer, file size, an optional
pre-download URL resolver, and an optional post-download effect. Pixiv ugoira
is the clearest example that “original URL” can require a site-specific
resolution and transformation workflow.

These are product/runtime observations. They should not become one mandatory
Dreamland capability with nullable methods for every site.

## Representative source traces

These traces prevent the family labels from hiding materially different
contracts:

The Yande trace in this section is pinned MoeLoaderP behavior (`post.xml` and
`tag.xml`). The live Dreamland v1 profile is JSON-based and separately backed
by the read-only Yande observations in `MOEBOORU-UX-LEARNINGS.md`.

- **Yande:** `YandeSite` detects the current `user_info` cookie (and accepts a
  legacy `user_id` cookie), requests tag hints from
  `tag.xml`, and requests posts from `post.xml` with `page`, `limit`, and a
  site tag expression. Its class does not override `ThumbAsync` or
  `StarAsync`; account detection therefore does not imply an online mutation
  capability.
- **Pixiv:** `PixivSite` chooses among new/tag, author, and rank routes. It
  carries a site cursor (`lastId`), creates children for multi-page
  illustrations, and defers detail loading. Ugoira items resolve metadata and
  attach post-download processing, so a gallery result and a downloadable
  file are separate lifecycle stages.
- **Sankaku Chan:** `SankakuChanSite` stores an access token in per-site
  settings, imports cookies from multiple domains, uses `meta.next` as a
  continuation cursor, accepts multi-keyword searches, and implements an
  online favorite action. This is the clearest evidence that auth state and
  site actions affect the effective capability set.
- **DeviantArt:** `DeviantartSite` refuses search without a logged-in cookie
  session and returns `nextCursor`. Its latest/search/popular categories are
  different routes, so “query” alone does not describe the complete browse
  contract.
- **CustomSite:** `CustomSiteConfig` can define login URL and cookie key,
  category discovery, first/follow-up/search API templates, XPath extraction,
  detail pages, child pagination, referers, and deferred URL resolution. It
  is an extension/configuration system, not evidence that Dreamland should
  expose arbitrary XPath execution as a normal site capability.

The relevant implementations are pinned in
`MoeLoaderP.Core/Sites/YandeSite.cs`, `PixivSite.cs`,
`SankakuChanSite.cs`, `DeviantartSite.cs`, `CustomSite.cs`, and
`CustomSiteConfig.cs`.

## API v1 consequences

The matrix supports this preliminary capability algebra:

### Stable core

Every registered site must expose:

1. stable identity and a human-readable descriptor;
2. one query operation returning normalized post summaries;
3. cancellation and operation identity supplied by the runtime;
4. an opaque continuation model for “next results”; and
5. media candidates that remain site/runtime-owned until download.

The core query must not assume random page-number navigation, portable tag
syntax, totals, a single image per post, or that all media URLs are immediately
downloadable.

### Optional site capabilities

The following should be separate capabilities, advertised dynamically by the
site descriptor and effective configuration/auth state:

- `TagSuggestionCapability` — autocomplete or tag metadata;
- `PageIndexCapability` — random-access page numbers, distinct from “next”;
- `PostLookupCapability` — hydrate one known site-scoped post reference;
- `CreatorCapability` — resolve a site-scoped artist/author profile;
- `PostDetailCapability` — resolve deferred metadata/media for a result;
- `CollectionCapability` — enumerate child media for a gallery/set post;
- `MediaResolutionCapability` — refresh/resolve a download source or apply a
  site-specific transform;
- `PostActionCapability` — site mutations such as favorite/star/vote;
- `RemoteFavoriteCapability` — favorite or unfavorite a post in a site’s
  remote favorite resource;
- `RemoteCollectionCapability` — enumerate remote collections and their posts;
- `CollectionDownloadCapability` — download a site-owned collection archive
  such as Yande’s pool ZIP; this is distinct from per-post downloads and is
  supported by live Yande behavior even though it is not a MoeLoaderP core
  trait;
- `CategoryCapability` — site-owned browse/category menus;
- `MirrorCapability` — alternate endpoint selection with equivalent semantics;
- `SiteAuth` — site-specific login/session lifecycle.

The runtime should return an explicit unsupported-capability error when a
caller requests one of these and the selected site does not advertise it.

### Site configuration

MoeLoaderP demonstrates three different configuration layers that Dreamland
should keep separate:

1. **Descriptor/config schema:** display name, enabled state, endpoint/mirror,
   supported query controls, page-size bounds, category definitions, and
   site-specific non-secret options.
2. **Runtime policy:** proxy, timeout, retry, safe-content policy, download
   directory, concurrency, and local filtering. These belong to Dreamland,
   not an adapter’s opaque config.
3. **Secrets/session state:** browser cookies, tokens, OAuth/device state, or
   credentials. These are secret-store references and auth state, never TOML
   values returned to the frontend.

The API needs a typed common schema for layer 1 plus an opaque, versioned
site extension for site-specific fields. It must also expose the
effective capability set after config and auth are applied; static booleans
are insufficient for sites such as Pixiv, Sankaku, and CustomSite.

### Authentication

Authentication is not one universal mechanism. The source contains browser
cookie import/detection, JWT/cookie sessions, token-based API access, and
site-specific login flows. `SiteAuth` should therefore model status
and challenges rather than accept a universal username/password pair:

- anonymous, authenticated, expired, or action-required state;
- browser-session import where applicable;
- site-specific interactive/device/OAuth challenge data;
- secret-store-backed credential/session references;
- logout/revoke where supported.

Search may require authentication for one site and not another; favorite
or other mutations may require a stricter auth state than browsing. The
descriptor must advertise those requirements per operation.

The `SankakuIdolSite` source contains bundled account material and query
credentials. That is evidence for an adapter-specific auth flow and a security
boundary, not a pattern Dreamland may carry forward.

## Open questions before API approval

1. Should the core continuation be a runtime-owned opaque handle, or a
   serializable site cursor sealed by the runtime?
2. Should deferred detail/children be separate traits, or one hydration trait
   with typed expansion requests?
3. Is media resolution part of site output, or a download-only capability
   that prevents frontend exposure of unstable URLs?
4. Which common query filters deserve typed fields, and which stay in a
   site expression/extension map?
5. What is the minimum config-schema format that supports CustomSite without
   making arbitrary HTML scraping part of the stable core?
6. Which auth challenges can be represented generically without leaking
   cookies, tokens, or site-specific secrets through Tauri?

## Source map

Key pinned files:

- `MoeLoaderP.Core/SiteManager.cs` — registry and custom-site loading;
- `MoeLoaderP.Core/Sites/MoeSite.cs` — base lifecycle and optional hooks;
- `MoeLoaderP.Core/Sites/MoeSiteHelper.cs` — capability/config/category model;
- `MoeLoaderP.Core/Sites/BooruSite.cs` — shared Booru parsing and URL variants;
- `MoeLoaderP.Core/Sites/CustomSiteConfig.cs` and `CustomSite.cs` —
  declarative site/config/auth surface;
- `MoeLoaderP.Core/SearchPara.cs` and `SearchedPage.cs` — query and
  continuation shapes;
- `MoeLoaderP.Core/MoeItem.cs` and `MoeItemHelper.cs` — metadata, children,
  variants, resolution, and post-download effects;
- `MoeLoaderP.Core/Sites/PixivSite.cs`, `SankakuChanSite.cs`,
  `DeviantartSite.cs`, and `YandeSite.cs` — representative API/auth/action
  implementations.

## Yande v1 feature profile

The first concrete Dreamland site profile now includes these user-facing
operations:

- **Tag search:** use the Moebooru post-list query with a site tag
  expression, page number, and page size. The live JSON API supports both the
  legacy array response and the `api_version=2` `{ "posts": [...] }` envelope.
- **Popular browse:** expose `day`, `week`, and `month` as explicit browse
  modes mapped to the ordinary `/post.json` query with `order:score` and a
  date expression. Day is one date, week is the Monday-Sunday window, and
  month is the calendar-month window. `page` and `limit` remain active so the
  selected window supports infinite scroll, matching the vendored MoeBooru
  `ImageDataSource` behavior.
- **Login:** open/import the Yande browser session at `/user/login`, detect
  the `user_info` cookie, and keep the cookie session in the secret/session
  boundary. No raw cookie is returned to React or persisted in ordinary TOML.
- **Favorite mutation:** expose add/remove favorite only when authenticated.
  Yande’s current web client uses `POST /post/vote.json` with `id` and
  `score=3` to add a favorite and `score=2` to remove the favorite. This is a
  site adapter detail behind a stable `RemoteFavoriteCapability` operation.

Yande's raw `/post/popular_by_day.json`, `/post/popular_by_week.json`, and
`/post/popular_by_month.json` endpoints are not ordinary page-number
pagination: live requests returned fixed 40-item windows even when `page` and
`limit` were supplied. Dreamland therefore follows MoeBooru's scrollable
behavior through `/post.json` date-plus-score queries, while keeping remote
favorites distinct from Dreamland's local saved items.

# MoeBooru parity plan

Status: implementation audit, 2026-08-11. The reference is the vendored
MoeBooru checkout at `vendor/moebooru`, not an external application contract.
This matrix compares user-visible behavior and deliberately translates
Android-only actions into desktop equivalents where that is meaningful.

## Parity boundary

“Full parity” means that a desktop user can perform the reference app’s core
explore, inspect, save-query, account, and download workflows. It does not
mean copying Android activities, Material transitions, notification channels,
wallpaper intents, or the reference app’s unverified implementation quirks.

Yande-specific pools and ZIP archives remain Dreamland extensions. They are
not required to make the MoeBooru exploration model complete.

## Current matrix

| Reference behavior | MoeBooru evidence | Dreamland now | Gap / decision |
| --- | --- | --- | --- |
| Default home has popular plus saved query tabs | [`MainActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/MainActivity.kt:55), [`Db.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Db.kt:52) | Latest, Popular, and SQLite-backed saved query tabs | **Covered:** pinned groups have explicit earlier/later controls; query editing remains |
| Popular day/week/month/year and all-time views | [`MainActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/MainActivity.kt:78), [`PopularActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PopularActivity.kt:123) | Day/week/month score-ranked paging | **Partial:** add year/all where Yande supports it; preserve infinite scroll |
| Popular date navigation and date picker | [`PopularActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PopularActivity.kt:90) | Day/week/month period, normalized date anchor, earlier/later navigation | **Partial:** year/all-time are not in the current Yande release gate |
| Infinite post paging | [`MainActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/MainActivity.kt:298) | Latest/search/popular use runtime sessions and UI sentinel | Covered after `34aacef`; needs live visual acceptance |
| Remote tag autocomplete | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:868) | Yande `/tag.json` suggestions after one character | **Partial:** add alias/type presentation and negative-token-aware selection |
| Advanced query editor | [`QueryActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/QueryActivity.kt:141), [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:621) | Free-form tags plus a Yande-serialized typed builder for rating, order, date, dimensions, score, ID, vote, megapixels, MD5, source, parent, pool, and user; saved queries can reopen and update generated fields | **Partial:** aliases and typed AST/serializers are still required before multi-site reuse |
| Saved query name/pin/reorder/delete | [`QueryActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/QueryActivity.kt:110), [`SavedActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/SavedActivity.kt:38) | SQLite definitions, name/pin/delete, stable IDs, site isolation, restart persistence, and navigation | **Partial:** explicit reorder UI remains; unsupported source variants are shown unavailable |
| Typed tags and tag-driven search | [`PreviewActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PreviewActivity.kt:350) | Tags are shown and clickable in cards/detail | **Partial:** typed category styling and complete detail tag actions |
| Configurable grid columns and card info | [`SettingsActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/SettingsActivity.kt:112), [`MainActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/MainActivity.kt:366) | Fixed responsive CSS grid | **Missing:** user-controlled column density and metadata visibility |
| Sample/preview quality preferences | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:111), [`SettingsActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/SettingsActivity.kt:154) | Settings expose full/sample/preview download quality; detail uses sample first with preview/full fallback | **Partial:** preview-quality preference is not yet separate from the display fallback |
| Full-screen preview pager | [`PreviewActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PreviewActivity.kt:98) | Focused post inspector with sample-first image, previous/next controls, position, Escape, and left/right keyboard navigation over loaded posts | **Partial:** no swipe gesture or full-screen route; loaded-post navigation is covered |
| Post detail metadata and actions | [`PreviewActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PreviewActivity.kt:222) | Image, tags, author/creator ID, site post route, source, checksum, parent/children state with exact search actions, rating, dimensions, download, favorite | **Partial:** similar search, author profile route, and other native actions |
| Similar-image search | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:316), `SimilarActivity.kt` | No similar command or UI | **Missing:** optional site capability and result view |
| Artist/user profile exploration | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:312), `UserActivity.kt` | Author has an exact `user:<name>` search action; artist tags remain separate | **Partial:** native account/profile route and artist profile exploration remain |
| Remote favorite mutation/list | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:391), `UserActivity.kt` | Add/remove and favorite page exist | **Partial:** verify live favorite-list semantics and reflect state after refresh |
| Vote detail and score actions | [`UserActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/UserActivity.kt:372) | Favorite maps to Yande score 3/2 | **Deferred:** vote breakdown is not required by the current Yande release gate |
| Login/register/reset/account editing | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:986) | Browser-session login/logout only | **Intentional desktop scope:** keep browser auth; do not handle passwords in Dreamland |
| Download queue, duplicate handling, history | [`Model.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/Model.kt:35), `MainActivity.kt` | Async SQLite queue, collision-safe path, history, open file | Covered with a stronger desktop-local policy |
| Crop, wallpaper, avatar, Android share intents | [`PreviewActivity.kt`](../vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PreviewActivity.kt:250), `CropActivity.kt` | No desktop equivalent | **Translate selectively:** native open/share; defer wallpaper/avatar/crop until requested |
| Theme, cache, column, info, page, notification settings | [`preferences.xml`](../vendor/moebooru/app/src/main/res/xml/preferences.xml) | Content policy, path, proxy, retry settings | **Partial:** add theme/grid/preview/cache controls that improve desktop use |
| Pool browsing | `Model.kt` pool query fields and Moebooru pool API | Yande and Konachan searchable public pool lists, ordered posts, and site-gated Yande ZIP queue | Dreamland extension; Yande ZIP auth/content-type acceptance and archive-history pagination remain release-gate work |

## Implementation order

The parity work should land as thin vertical slices:

1. **Exploration shell:** local saved-query schema/commands, pinned query tabs,
   and explicit popular date navigation.
2. **Query builder:** typed filters mapped to the existing site-neutral query
   expression, with Yande fixture coverage for each supported field.
3. **Inspection:** focused preview route, adjacent-post navigation, complete
   metadata/tag actions, and a similar-image capability when Yande’s method is
   verified.
4. **Presentation settings:** grid density, card metadata, preview quality,
   and theme/cache preferences.
5. **Identity exploration:** author/artist routes only after live Yande
   identity semantics are verified; browser auth remains the credential
   boundary.

Each slice must preserve the existing Yande release gate and pass `make check`.
Android-only actions remain explicit deferrals rather than silently becoming
fake desktop features.

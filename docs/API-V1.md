# Dreamland API v1

Status: implementation baseline, 2026-08-11. The core site/runtime boundary
is accepted for incremental implementation; the review gaps at the end of
this document remain explicit follow-up decisions.

## Naming decision

The domain term is **site**, matching MoeLoaderP's `MoeSite` and
`SiteManager`. A site may be an image board such as Yande, an art service such
as Pixiv, or a social media service such as Twitter. `source` remains reserved
for an artwork's original/source URL.

## Objective

Define one stable Rust/runtime boundary that can represent the sites in
the pinned MoeLoaderP checkout while giving Yande a complete first release:

- browse and site tag-expression search;
- popular-by-day/week/month windows;
- pool metadata, ordered pool posts, and site archive downloads;
- tag suggestions;
- normalized posts with media variants and optional children;
- browser-session authentication;
- remote favorite add/remove and an authenticated favorite page; and
- reference-based downloads owned by the runtime;
- download history and replayable saved-query history backed by
  SQLite; and
- explicit mapping between local state and site-remote capabilities.

React must not know site response JSON, site URL construction, cookie
formats, pagination internals, or filesystem paths.

The release scope and evidence are tracked in
[YANDE-RELEASE-GATE.md](YANDE-RELEASE-GATE.md). The site inventory is in
[MOELOADER-SITE-MATRIX.md](MOELOADER-SITE-MATRIX.md).

## Evidence boundary

The design is based on Dreamland's nested MoeLoaderP submodule:

    vendor/moeloaderp
    commit 0025dd999306258103ed2b239b2132d3913c99e0

Observed shapes:

- MoeSite requires one asynchronous page-fetch operation and has optional
  autocomplete, account, favorite, category, and detail hooks.
- SearchPara mixes site intent with local filtering, page numbers, cursors,
  categories, sort, date, and per-site configuration.
- SearchedPage can carry page numbers, cursors, optional totals, and a next
  request.
- MoeItem can represent one image, a gallery parent, deferred detail, or a
  site-specific media workflow.
- UrlInfo carries a media kind, referer, file size, pre-download resolution,
  and post-download processing.
- Sites include Booru XML/JSON adapters, page-numbered HTML/API sites,
  cursor APIs, authenticated sites, gallery sites, and a declarative
  custom-site adapter.

Yande provides tag-expression post queries, tag hints, three popular endpoints,
direct media variants, browser-cookie detection, and web favorite mutations
through /post/vote.json. The current popular endpoints return fixed-size result
windows and do not honor ordinary page/limit navigation, but their date
parameters select different day/week/month windows.

Konachan is the second real Moebooru adapter. Its primary API and browser
origin are `https://konachan.com`; the adapter accepts the full
`ContentPolicy` range. If the primary API is challenged by bot protection,
safe-only requests may retry against the separate G-rated
`https://konachan.net` mirror. Non-safe requests never use that fallback and
surface the `.com` browser recovery route instead.

## Boundaries

- React owns rendering, interaction state, and view state only.
- Rust owns site HTTP, decoding, normalization, auth/session integration,
  capability checks, config validation, download I/O, and errors.
- Tauri commands expose application concepts, never site JSON or arbitrary
  network/filesystem primitives.
- A site reference is (site_id, post_id), never a bare remote ID.
- A frontend URL, referer, filename, or path is never download authority.
- Credentials, cookies, tokens, and continuation state do not enter ordinary
  TOML or frontend state.
- SQLite-backed local state and site-remote state are different resources.
  Local state is always available; remote state is capability- and auth-bound.
- A saved query is a local replayable search definition, not a saved post.
- A remote favorite is site state; it is never silently mirrored into a
  local post-bookmark collection.
- Site syntax may be preserved as an opaque expression, but the API does
  not claim that tags, ratings, sorting, or pagination mean the same thing
  across sites.

## Domain model

The following is conceptual Rust. Exact derives and wire representation are
implementation details; the ownership and semantics are part of v1.

~~~rust
pub struct SiteId(String);
pub struct PostId(String);

pub struct PostRef {
    pub site: SiteId,
    pub id: PostId,
}

pub struct CreatorRef {
    pub site: SiteId,
    pub id: String,
}

pub struct CreatorProfile {
    pub reference: CreatorRef,
    pub name: String,
    pub handle: Option<String>,
    pub profile_url: Option<String>,
    pub description: Option<String>,
}

pub struct RemoteCollectionRef {
    pub site: SiteId,
    pub id: String,
}

pub struct RemoteCollection {
    pub reference: RemoteCollectionRef,
    pub name: String,
    pub kind: RemoteCollectionKind,
    pub writable: bool,
    pub visibility: CollectionVisibility,
    pub post_count: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub enum CollectionVisibility {
    Public,
    Private,
    Unknown,
}

pub struct RemoteCollectionPage {
    pub collections: Vec<RemoteCollection>,
    pub continuation: Continuation,
    pub total: Option<u64>,
    pub page_size: u16,
    pub session: Option<QuerySessionId>,
}

pub struct SavedQuery {
    pub id: LocalRecordId,
    pub site: SiteId,
    pub request: ReplayableQuery,
    pub name: String,
    pub pinned: bool,
    pub sort_order: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct DownloadRecord {
    pub id: LocalRecordId,
    pub target: DownloadTarget,
    pub metadata: Option<DownloadMetadata>,
    pub status: DownloadStatus,
    pub path: Option<String>,
    pub checksum: Option<Checksum>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_code: Option<ErrorCode>,
}

pub struct DownloadMetadata {
    pub tags: Vec<PostTag>,
    pub posting_account: Option<RemoteUser>,
    pub source: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

pub enum DownloadTarget {
    Post {
        reference: PostRef,
        variant: Option<MediaVariantKind>,
    },
    CollectionArchive {
        collection: RemoteCollectionRef,
        format: ArchiveFormat,
    },
}

pub enum DownloadOrganization {
    Site,
    Flat,
}

pub enum ArchiveFormat {
    Zip,
    SiteDefined(String),
}

pub enum DownloadStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    ExistingTarget,
}

pub struct QueryHistoryEntry {
    pub id: LocalRecordId,
    pub site: SiteId,
    pub source: DiscoverySource,
    pub created_at: DateTime<Utc>,
}

pub struct LocalRecordId(String);

pub struct Post {
    pub reference: PostRef,
    pub content_kind: ContentKind,
    pub title: Option<String>,
    pub text: Option<String>,
    pub tags: Vec<PostTag>,
    pub rating: ContentRating,
    pub description: Option<String>,
    pub detail_url: Option<String>,
    pub creators: Vec<CreatorRef>,
    pub score: Option<i64>,
    pub favorite_count: Option<u64>,
    pub remote_favorite: FavoriteStatus,
    pub posting_account: Option<RemoteUser>,
    pub source: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub checksum: Option<Checksum>,
    pub media: Vec<MediaVariant>,
    pub children: ChildrenState,
    pub detail: DetailState,
}

pub enum ContentKind {
    Image,
    Gallery,
    Animation,
    Video,
    MixedMedia,
    TextWithMedia,
    Unknown,
}

pub enum RemoteCollectionKind {
    Pool,
    Favorites,
    Bookmarks,
    Album,
    Folder,
    SiteDefined(String),
}

pub struct PostTag {
    pub name: String,
    pub category: Option<TagCategory>,
    pub post_count: Option<u64>,
    pub site_value: Option<String>,
}

pub struct RemoteUser {
    pub id: Option<String>,
    pub name: String,
}

pub struct ContentRating {
    pub level: RatingLevel,
    pub site_value: Option<String>,
}

pub enum RatingLevel {
    Safe,
    Questionable,
    Explicit,
    Unknown,
}

pub struct Checksum {
    pub algorithm: String,
    pub value: String,
}

pub enum FavoriteStatus {
    Unknown,
    NotFavorited,
    Favorited,
}

pub enum TagCategory {
    Artist,
    Character,
    Copyright,
    General,
    Meta,
    Unknown,
}

pub enum ChildrenState {
    NotSupported,
    NotLoaded { count: Option<u32> },
    Loaded {
        posts: Vec<Post>,
        next: Option<ContinuationToken>,
        total: Option<u64>,
    },
}

pub enum DetailState {
    Deferred,
    Partial {
        metadata: LoadState,
        media: LoadState,
        children: LoadState,
    },
    Complete,
}

pub enum LoadState {
    Loaded,
    Deferred,
    Unavailable,
}

pub struct MediaVariant {
    pub kind: MediaVariantKind,
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: Option<u64>,
    pub extension: Option<String>,
    pub referer: Option<String>,
    pub requires_resolution: bool,
    pub content_type: Option<String>,
    pub duration_ms: Option<u64>,
    pub post_download: Option<PostDownloadEffect>,
}

pub enum PostDownloadEffect {
    None,
    Transform {
        effect_id: String,
        output_extension: Option<String>,
    },
    SiteDefined(String),
}

pub enum MediaVariantKind {
    Preview,
    Sample,
    Large,
    Original,
    Animation,
    Video,
    Audio,
    Document,
    SiteDefined(String),
}
~~~

Rules:

- PostId is a string because site IDs are not universally numeric.
- Missing metadata remains None or Unknown; adapters must not invent it.
- Tags preserve site spelling. Category and count are optional metadata.
- Remote favorite state may be Unknown in anonymous list responses.
- Media URLs are site output consumed by runtime rendering/download.
  React cannot submit or replace them in a download command.
- ChildrenState and DetailState make deferred work explicit. ChildrenState
  carries the current child page and its internal continuation; an empty vector
  does not mean the site has no children. DetailState is
  per expansion, so metadata, media, and children can be loaded independently.
- SiteDefined variants are not selectable until advertised by the site
  capability/configuration.
- A resolvable variant is resolved immediately before download and may return
  a new source plus a PostDownloadEffect. The runtime owns that lifecycle and
  never asks React to execute a site transform.

### Shared parent models and site implementations

The conceptual `Post` above is not meant to become one enormous adapter DTO.
The implementation should compose a few reusable parent models, similar to the
useful shared fields in MoeBooru's `JImageItem` and MoeLoaderP's `MoeItem`:

~~~rust
pub struct PostSummary {
    pub identity: PostIdentity,
    pub classification: PostClassification,
    pub attribution: PostAttribution,
    pub stats: PostStats,
    pub media: Vec<MediaVariant>,
}

pub struct PostIdentity {
    pub reference: PostRef,
    pub detail_url: Option<String>,
    pub checksum: Option<Checksum>,
}

pub struct PostClassification {
    pub content_kind: ContentKind,
    pub tags: Vec<PostTag>,
    pub rating: ContentRating,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub struct PostAttribution {
    pub posting_account: Option<RemoteUser>,
    pub creators: Vec<CreatorRef>,
    pub source: Option<String>,
}

pub struct PostStats {
    pub score: Option<i64>,
    pub favorite_count: Option<u64>,
    pub remote_favorite: FavoriteStatus,
}

pub struct PostDetail {
    pub summary: PostSummary,
    pub title: Option<String>,
    pub text: Option<String>,
    pub description: Option<String>,
    pub children: ChildrenState,
}
~~~

These are composition boundaries, not inheritance requirements. A site adapter
may fill only the fields it actually knows. The runtime owns the common
validation, safe-content policy, local metadata snapshot, media selection, and
download queue; it does not make site-specific fields mandatory.

The normalization pipeline is:

    site wire response -> site DTO -> site mapper -> PostSummary/PostDetail
    -> runtime policy -> Tauri DTO

For example, Yande maps its `author` and `creator_id` fields to
`PostAttribution.posting_account`, while typed artist tags remain
`PostClassification.tags`. Pixiv can map a multi-page illustration to the
same summary plus children/detail/media-resolution capabilities. A future
Twitter adapter can map a post with different actions without adding a
Twitter branch to the common post model.

## Query and pagination

The common query carries intent, an opaque site expression, runtime-safe
filters, and a pagination request.

~~~rust
pub struct PostQueryRequest {
    pub query: ReplayableQuery,
    pub pagination: PaginationRequest,
}

pub struct ReplayableQuery {
    pub source: DiscoverySource,
    pub category: Option<String>,
    pub content_policy: ContentPolicy,
    pub local_filter: LocalPostFilter,
    pub sort: Option<SortRequest>,
    pub date: Option<DateRange>,
}

pub enum DiscoverySource {
    Browse,
    Search { expression: String },
    Feed { kind: FeedKind },
    Creator { creator: CreatorRef },
    Collection { collection: RemoteCollectionRef },
}

pub enum FeedKind {
    Latest,
    Popular { period: PopularPeriod, anchor_date: NaiveDate },
    Ranking { period: Option<String>, category: Option<String> },
    SiteDefined(String),
}

pub enum PopularPeriod {
    Day,
    Week,
    Month,
}

pub struct LocalPostFilter {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub orientation: Option<ImageOrientation>,
    pub extensions: Vec<String>,
    pub min_score: Option<i64>,
}

pub enum ImageOrientation {
    Landscape,
    Portrait,
    Square,
}

pub struct SortRequest {
    pub key: String,
    pub descending: bool,
}

pub struct DateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub enum PaginationRequest {
    First { page_size: u16 },
    Page { number: NonZeroU32, page_size: u16 },
    Continue { token: ContinuationToken },
    FixedWindow,
}

pub struct SitePage {
    pub posts: Vec<Post>,
    pub continuation: Continuation,
    pub total: Option<u64>,
    pub page_size: u16,
}

pub struct PostPage {
    pub posts: Vec<Post>,
    pub total: Option<u64>,
    pub page_size: u16,
    pub session: Option<QuerySessionId>,
    pub has_next: bool,
}

pub enum Continuation {
    None,
    Next(ContinuationToken),
}

pub struct ContinuationToken(String);
pub struct QuerySessionId(String);

pub enum ContentPolicy {
    SafeOnly,
    AllowQuestionable,
    AllowExplicit,
    ExplicitOnly,
}
~~~

Semantics:

- TagSearch uses a site expression. For Yande this is the Booru expression,
  including meta-tags and rating syntax.
- Popular is a first-class feed, not sort=popular. Yande maps each
  `PopularPeriod` and `anchor_date` to a separate endpoint/date parameter and
  returns no continuation for each fixed-size window. There is no generic year
  mode until a concrete site advertises one.
- First, Page, and Continue are distinct capabilities. A cursor-only site
  must not pretend to support random page numbers.
- Site continuation tokens are internal runtime state. The runtime seals them
  to site, query fingerprint, auth/session identity, and expiry; it
  never places them in PostPage or a Tauri argument.
- QuerySessionId is a frontend-safe runtime handle, not a site cursor.
  `start_query` creates it, `continue_query` advances it, and `cancel_query`
  releases it. The runtime owns expiry, cancellation, and the site's next
  request.
- ContentPolicy is a runtime safety requirement. An adapter may add a site
  filter but may not weaken it.
- Sort and date are optional intent. Sites advertise supported values and
  reject unsupported ones rather than silently ignoring them.
- A changed page size starts a new query session. Results are deduplicated by
  PostRef within a session, but no remote snapshot is promised.

## Optional site capabilities

PostQueryCapability is required for every registered site. The other
capabilities are explicit object-safe traits; unsupported operations are not
fake methods returning empty success.

~~~rust
pub struct RequestContext {
    pub operation_id: OperationId,
    pub cancellation: CancellationToken,
    pub session: Option<SiteSessionHandle>,
}

pub struct OperationId(String);
pub struct SiteSessionHandle(String);

pub struct SiteError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

// SiteError is internal to the adapter/runtime boundary. The runtime
// adds operation_id and maps it to RuntimeError before crossing Tauri.

#[async_trait]
pub trait PostQueryCapability: Send + Sync {
    async fn query_posts(
        &self,
        request: PostQueryRequest,
        context: RequestContext,
    ) -> Result<SitePage, SiteError>;
}

#[async_trait]
pub trait TagSuggestionCapability: Send + Sync {
    async fn suggest_tags(
        &self,
        request: TagSuggestionRequest,
        context: RequestContext,
    ) -> Result<Vec<TagSuggestion>, SiteError>;
}

pub struct TagSuggestionRequest {
    pub query: String,
    pub limit: u16,
}

pub struct TagSuggestion {
    pub name: String,
    pub category: Option<TagCategory>,
    pub post_count: Option<u64>,
    pub ambiguous: Option<bool>,
    pub aliases: Vec<String>,
}

#[async_trait]
pub trait RelatedTagCapability: Send + Sync {
    async fn related_tags(
        &self,
        request: RelatedTagRequest,
        context: RequestContext,
    ) -> Result<Vec<RelatedTag>, SiteError>;
}

pub struct RelatedTagRequest {
    pub tags: Vec<String>,
    pub limit: u16,
}

pub struct RelatedTag {
    pub name: String,
    pub post_count: Option<u64>,
}

#[async_trait]
pub trait PostLookupCapability: Send + Sync {
    async fn lookup(
        &self,
        reference: &PostRef,
        context: RequestContext,
    ) -> Result<Post, SiteError>;
}

#[async_trait]
pub trait PostDetailCapability: Send + Sync {
    async fn hydrate(
        &self,
        reference: &PostRef,
        expansion: DetailExpansion,
        context: RequestContext,
    ) -> Result<Post, SiteError>;
}

pub enum DetailExpansion {
    Metadata,
    Media,
    Children,
    All,
}

#[async_trait]
pub trait RemoteFavoriteCapability: Send + Sync {
    async fn set_saved(
        &self,
        reference: &PostRef,
        collection: Option<&RemoteCollectionRef>,
        saved: bool,
        context: RequestContext,
    ) -> Result<SaveMutation, SiteError>;
}

pub struct SaveMutation {
    pub reference: PostRef,
    pub collection: Option<RemoteCollectionRef>,
    pub saved: bool,
}

#[async_trait]
pub trait RemoteCollectionCapability: Send + Sync {
    async fn list_collections(
        &self,
        pagination: PaginationRequest,
        context: RequestContext,
    ) -> Result<RemoteCollectionPage, SiteError>;

    async fn list_collection_posts(
        &self,
        collection: Option<&RemoteCollectionRef>,
        pagination: PaginationRequest,
        context: RequestContext,
    ) -> Result<SitePage, SiteError>;
}

#[async_trait]
pub trait RemoteFavoriteListCapability: Send + Sync {
    async fn list_favorites(
        &self,
        pagination: PaginationRequest,
        context: RequestContext,
    ) -> Result<SitePage, SiteError>;
}

#[async_trait]
pub trait CollectionDownloadCapability: Send + Sync {
    async fn download_collection(
        &self,
        collection: &RemoteCollectionRef,
        format: ArchiveFormat,
        context: RequestContext,
    ) -> Result<SiteDownload, SiteError>;
}

pub struct SiteDownload {
    pub collection: RemoteCollectionRef,
    pub format: ArchiveFormat,
    pub media: MediaVariant,
}

#[async_trait]
pub trait CreatorCapability: Send + Sync {
    async fn get_creator(
        &self,
        reference: &CreatorRef,
        context: RequestContext,
    ) -> Result<CreatorProfile, SiteError>;
}

#[async_trait]
pub trait CollectionCapability: Send + Sync {
    async fn list_children(
        &self,
        reference: &PostRef,
        pagination: PaginationRequest,
        context: RequestContext,
    ) -> Result<SitePage, SiteError>;
}

#[async_trait]
pub trait MediaResolutionCapability: Send + Sync {
    async fn resolve_media(
        &self,
        reference: &PostRef,
        variant: MediaVariantKind,
        context: RequestContext,
    ) -> Result<MediaVariant, SiteError>;
}

#[async_trait]
pub trait PostActionCapability: Send + Sync {
    fn actions(&self) -> Vec<PostActionDescriptor>;
    async fn execute(
        &self,
        reference: &PostRef,
        action: &str,
        context: RequestContext,
    ) -> Result<PostActionResult, SiteError>;
}

pub struct PostActionDescriptor {
    pub id: String,
    pub label: String,
    pub reversible: bool,
}

pub struct PostActionResult {
    pub action: String,
    pub reference: PostRef,
    pub changed: bool,
}

pub struct PostActionInput {
    pub reference: PostRef,
    pub action: String,
}

#[async_trait]
pub trait CategoryCapability: Send + Sync {
    async fn categories(
        &self,
        context: RequestContext,
    ) -> Result<Vec<CategoryNode>, SiteError>;
}

pub struct CategoryNode {
    pub id: String,
    pub label: String,
    pub children: Vec<CategoryNode>,
}

#[async_trait]
pub trait MirrorCapability: Send + Sync {
    async fn mirrors(
        &self,
        context: RequestContext,
    ) -> Result<Vec<MirrorDescriptor>, SiteError>;
}

pub struct MirrorDescriptor {
    pub id: String,
    pub label: String,
    pub base_url: String,
}
~~~

PostLookupCapability means a reference can be queried directly.
PostDetailCapability means a list result can be hydrated. They are distinct:
some sites expose IDs but not lookup, while others reveal original media
only on a detail request.

CollectionCapability is separate because child enumeration can have its own
pagination. MediaResolutionCapability is separate because animated or protected
media can require a refresh or post-download transform.

RemoteFavoriteCapability and RemoteCollectionCapability are separate because a
site may support favoriting without exposing a list, or expose collections only
after auth. A site collection can be a pool, album, bookmark list, or
favorite list; it is not the same thing as a local SavedQuery.
Yande's release requires both after the authenticated read mechanism is
verified.

`RemoteFavoriteListCapability` is intentionally separate from generic
collections. Yande v1 cannot advertise it until authorized verification proves
current-user scoping, private visibility, ordering, and continuation semantics.

RemoteFavoriteCapability is deliberately narrow for v1: it models Yande's
binary favorite mutation. Pixiv bookmarks and X bookmarks need separate
capabilities until their visibility, idempotency, and auth semantics are
verified as common.

CreatorCapability supplies a profile for a site-scoped creator reference;
creator post listings use the normal PostQueryCapability with
DiscoverySource::Creator.

LocalPostFilter is always enforced by the runtime after normalization. An
adapter may translate it into remote query parameters to reduce transfer, but
the remote result is still checked locally before it is returned.

CategoryCapability and MirrorCapability cover site-owned browse menus and
equivalent endpoint selection. They are not runtime policy: a site may
advertise neither, one, or both.

Site implementations use these contracts by composition. The Yande adapter
will implement the required post query plus tag suggestions, browser auth,
remote favorite mutation, public pool collections, and authenticated pool ZIP
download. Pixiv can implement query, creator/detail, child/media-resolution,
auth, and remote bookmark capabilities without implementing pool methods.
Twitter can use the same post/collection/download primitives where its API
supports them and expose network-specific actions only through optional
capabilities. Unsupported site capabilities are absent from the adapter and
the effective snapshot; they are not represented by empty placeholder methods.

PostActionCapability is the bounded escape hatch for site actions that are
not the common saved-resource operation. The frontend may render descriptors
and submit descriptor IDs, but it cannot invent arbitrary action IDs. Favorite
mutation remains RemoteFavoriteCapability so its state and idempotency semantics
stay common.

## Local state and feature mapping

Local state is a runtime service backed by SQLite. Sites never read or
write these tables directly, and local rows are not treated as proof of remote
state.

~~~rust
pub trait StateStore: Send + Sync {
    async fn save_query(
        &self,
        query: SavedQuery,
        context: RequestContext,
    ) -> Result<SavedQuery, RuntimeError>;
    async fn update_query(
        &self,
        query: SavedQuery,
        context: RequestContext,
    ) -> Result<SavedQuery, RuntimeError>;
    async fn delete_query(
        &self,
        id: &LocalRecordId,
        context: RequestContext,
    ) -> Result<(), RuntimeError>;
    async fn list_saved_queries(
        &self,
        pinned_only: bool,
        context: RequestContext,
    ) -> Result<Vec<SavedQuery>, RuntimeError>;
    async fn download_history(
        &self,
        pagination: PaginationRequest,
        context: RequestContext,
    ) -> Result<Vec<DownloadRecord>, RuntimeError>;
}
~~~

The minimum SQLite ownership boundary is:

| Table | Purpose | Authority |
| --- | --- | --- |
| `saved_queries` | Named/pinned site queries and display order | Dreamland |
| `download_history` | Queue outcome, path, checksum, error state, and immutable post metadata snapshot | Dreamland |
| `download_queue` | Durable pending and in-progress image jobs | Dreamland |
| `archive_queue` / `archive_history` | Durable pool archive jobs and terminal outcomes | Dreamland |

Rules:

- Saved queries contain site identity and `ReplayableQuery` intent, so replay
  always starts a new query session without relying on a current screen,
  page position, session ID, or opaque site cursor.
- Pin/order changes are local-only UI state. They do not change site query
  semantics. Reordering is atomic within one pin group.
- Download history records the local outcome and path; it is not a site
  feature and does not require authentication. Its tag snapshot supports
  local tag/account search and remains useful after the remote post changes.
- Query-session history and detail-image cache are runtime concerns, not
  SQLite tables in 0.1.0. Query sessions are replayable in memory and detail
  images use the XDG/platform cache directories documented by the runtime.
- Local rows contain no cookies, tokens, signed URLs, or site secrets.

The intended site mapping is:

| User job | Yande | Konachan | Pixiv | Possible X adapter |
| --- | --- | --- | --- | --- |
| Search | `Search` with tag expression | `Search` with tag expression | `Search` with tag expression | `Search` with site expression |
| Discover | `Feed(Popular { period, anchor_date })` | `Browse` or site feed | `Feed(Ranking)` | site feed, if supported |
| Creator browsing | `Creator` when creator/account lookup is available | `Creator` when supported | `Creator` and creator profile | `Creator` and profile, if supported |
| Open gallery | `Collection`/children | post media variants | children plus deferred detail | post media attachments |
| Save | remote favorite via `RemoteFavoriteCapability` | unsupported or site-defined | deferred bookmark capability | site-defined action |
| Download | image variants | image variants | image/gallery/animation variants | image/video variants, if supported |

The X column is intentionally conditional. A social site may use the same
discovery and media primitives while leaving social actions, text threads, and
network-specific collections as explicit site actions/extensions.

## Site descriptor and capabilities

The current implementation uses object-safe capability ports. A site adapter owns
its validated bundled defaults and returns optional ports only for features it
actually implements. The registry validates that descriptor flags agree with
those ports before the Tauri runtime starts.

~~~rust
pub trait SiteAdapter: Send + Sync {
    fn descriptor(&self) -> &SiteDescriptor;
    fn post_query(&self) -> &dyn PostQueryCapability;
    fn tag_suggestions(&self) -> Option<&dyn TagSuggestionCapability>;
    fn related_tags(&self) -> Option<&dyn RelatedTagCapability>;
    fn post_lookup(&self) -> Option<&dyn PostLookupCapability>;
    fn collections(&self) -> Option<&dyn CollectionCapability>;
    fn collection_download(&self) -> Option<&dyn CollectionDownloadCapability>;
    fn remote_favorites(&self) -> Option<&dyn RemoteFavoriteCapability>;
    fn remote_favorite_list(&self) -> Option<&dyn RemoteFavoriteListCapability>;
    fn current_user(&self) -> Option<&dyn CurrentUserCapability>;
    fn media_resolution(&self) -> &dyn MediaResolutionCapability;
    fn browser_routes(&self) -> &dyn BrowserRoutesCapability;
}

pub struct SiteRegistry {
    adapters: Vec<Box<dyn SiteAdapter>>,
}
~~~

The registry is the only composition root. Tauri and runtime callers resolve a
site once and invoke a capability port; they do not match site IDs or import a
concrete adapter. Unsupported capabilities are absent, not empty success
methods. Effective auth/config health snapshots remain a later extension of
the same ports rather than a second dispatch mechanism.

## Authentication

Authentication is a site capability, not a universal username/password
method. The runtime owns the secret/session store and browser integration.

~~~rust
pub enum AuthDescriptor {
    None,
    BrowserSession { login_url: String, cookie_domains: Vec<String> },
    Interactive { methods: Vec<AuthMethod> },
}

pub enum AuthMethod {
    BrowserSession,
    OAuth,
    DeviceCode,
    CredentialForm,
}

pub enum AuthStatus {
    Anonymous,
    Authenticated { account: AccountSummary },
    Expired,
    ActionRequired { reason: String },
}

pub struct AccountSummary {
    pub site_user_id: Option<String>,
    pub display_name: Option<String>,
}

pub struct AuthChallenge {
    pub challenge_id: String,
    pub method: AuthMethod,
    pub user_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait SiteAuth: Send + Sync {
    fn descriptor(&self) -> AuthDescriptor;
    async fn status(&self, context: RequestContext) -> Result<AuthStatus, SiteError>;
    async fn begin(&self, context: RequestContext) -> Result<AuthChallenge, SiteError>;
    async fn complete(
        &self,
        challenge_id: &str,
        context: RequestContext,
    ) -> Result<AuthStatus, SiteError>;
    async fn refresh(&self, context: RequestContext) -> Result<AuthStatus, SiteError>;
    async fn logout(&self, context: RequestContext) -> Result<(), SiteError>;
}
~~~

Rules:

- Yande uses BrowserSession with /user/login and current `user_info` cookie
  detection; the adapter also accepts the legacy `user_id` cookie.
  Browser integration imports session state into the runtime secret store;
  React receives only AuthStatus and safe challenge metadata.
- Site methods obtain a site-bound `SiteSession` from runtime context; no
  site capability accepts a free-standing cookie or password from a Tauri
  command. The runtime may unwrap the session only at the adapter/transport
  boundary.
- For BrowserSession, begin opens or returns the declared login URL,
  complete asks the runtime browser bridge to import the challenge's cookie
  domains, and refresh revalidates the imported session. The site never
  receives browser cookie material through the application wire type.
- Yande browsing is anonymous; favorite mutation and the favorite page require
  Authenticated.
- Logout clears site session state and effective capabilities but does not
  delete saved query definitions or download history.
- Auth failure is auth_required or auth_expired, not generic network/decode.

## Site configuration

Three owners remain separate.

Dreamland owns download path, concurrency, retry policy, proxy policy, safe
content policy, cache policy, and UI preferences.

The site owns a versioned non-secret extension schema:

~~~rust
pub struct SiteConfig {
    pub enabled: bool,
    pub extension_version: u32,
    pub extension: toml::Table,
}

pub struct RuntimeConfig {
    pub sites: std::collections::BTreeMap<SiteId, SiteConfig>,
    pub download: DownloadConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
}

pub struct RuntimeConfigInput {
    pub sites: std::collections::BTreeMap<SiteId, SiteConfigInput>,
    pub download: DownloadConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
}

pub struct DownloadConfig {
    pub directory: String,
    pub quality: DownloadQuality,
    pub concurrency: u16,
    pub organization: DownloadOrganization,
}

pub enum DownloadQuality {
    BestAvailable,
    Original,
    Efficient,
    Sample,
}

pub struct CacheConfig {
    pub max_bytes: u64,
    pub ttl_seconds: u64,
}

pub struct LoggingConfig {
    pub level: LogLevel,
}

pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub struct SiteConfigInput {
    pub enabled: bool,
    pub extension_version: u32,
    pub extension: toml::Table,
    pub secret_refs: std::collections::BTreeMap<String, SecretRef>,
}

pub struct SecretRef {
    pub key: String,
}

pub struct SiteConfigSchema {
    pub site: SiteId,
    pub version: u32,
    pub fields: Vec<ConfigField>,
}

pub struct ConfigField {
    pub key: String,
    pub kind: ConfigFieldKind,
    pub required: bool,
    pub secret: bool,
}

pub enum ConfigFieldKind {
    Text,
    Url,
    Boolean,
    Integer,
    Select { values: Vec<String> },
    Json,
}
~~~

Download selection is deterministic: an explicit `DownloadTarget::Post`
variant wins over `DownloadConfig.quality`; `None` uses the configured quality.
The v1 default is `BestAvailable`, ordered as Original, Large, Sample, then
Preview among variants the site actually advertises. A missing requested
variant is a stable unsupported-capability/media error, never a silent quality
downgrade.

Site extensions may describe endpoint mirrors, categories, page limits, or
site-specific query controls. Secret fields resolve through the secret
store and are never returned in SiteConfig wire responses.

The runtime TOML file uses common top-level settings plus
`[sites.<site-id>]` sections. The lifecycle is load -> validate -> apply -> persist. Validation checks the
schema version, field kinds, required fields, URL allowlists, and secret
references before the registry applies a new configuration. Applying a
configuration refreshes effective capabilities and invalidates affected query
sessions. Raw secret values can be written only through the runtime secret
store; SiteConfigInput accepts references, never secret text.

Yande v1 needs only a validated base URL/mirror override and runtime content
policy. Its default endpoint is https://yande.re. Login cookies are session
state, not site TOML values.

### Download layout and canonical names

The default organization is `Site`:

    <directory>/<site>/posts/<canonical-name>.<extension>

For a post, the canonical name is `<post-id>_<checksum>` when a site
checksum exists, otherwise `<post-id>`. The extension comes from the resolved
media content type, not from a frontend-supplied filename. For a site
archive, the default is:

    <directory>/<site>/pools/pool-<pool-id>_<safe-pool-name>.zip

The runtime normalizes Unicode, removes path separators and reserved names,
limits each component to a safe byte length, and falls back to the stable
site-scoped ID when metadata is missing. Existing canonical files are
never overwritten: if the target path exists, the queued item terminates as
`ExistingTarget` before writing. The user can explicitly choose another
destination later; the runtime does not silently deduplicate or rename it.

Tags and posting-account names are not used as directory keys. Tags are
mutable, numerous, and may contain several artist/character/copyright
categories. The post detail view and download history expose the complete
normalized tag list, and the runtime persists an immutable metadata snapshot
with the local download record. This makes tags searchable without making
mutable labels filesystem identity. The raw Yande/MoeBooru `author` field is
modeled as `posting_account`; it is not treated as the artwork's artist.
Artist identity comes from `PostTag.category == Artist` and may contain
multiple tags.

`Flat` remains available for users who need compatibility with an existing
folder, but is not the default. A later user-configurable template may offer
tag- or artist-oriented views without making those unstable values part of
canonical identity. Date remains metadata and an optional UI filter, not the
library's primary organization.

### Download queue and local cache

Downloads are durable queue records, not synchronous file operations. Enqueue
stores the target, resolved metadata, destination, and operation state in local
SQLite. A worker resolves media, streams into a temporary file under the local
cache, reports progress, then atomically renames the completed temporary file
to its canonical destination. The temporary cache is recoverable queue state,
not a user-managed library and not a scan of arbitrary existing files.

The worker checks the destination before opening it and again immediately before
the atomic rename. Any existing target terminates the item as `ExistingTarget`;
it never overwrites, merges, or guesses that two files are duplicates. Failed
temporary files are cleaned up or retained only when needed for explicit retry.

## Registry

~~~rust
pub struct SiteRegistry {
    active: Vec<Box<dyn SiteAdapter>>,
    skeletons: Vec<SiteDescriptor>,
}

impl SiteRegistry {
    pub fn site(&self, site_id: &str) -> anyhow::Result<&dyn SiteAdapter>;
    pub fn descriptors(&self) -> Vec<SiteDescriptor>;
    pub fn all_descriptors(&self) -> Vec<SiteDescriptor>;
    pub fn validate(&self) -> anyhow::Result<()>;
}
~~~

The runtime constructs the registry once. Unknown, disabled, unhealthy, and
unsupported sites are distinct runtime states. Adding an active site requires
registering an adapter in this composition root, but does not add site-shaped
branches to Tauri, runtime, or the frontend.

## Tauri command surface

Commands carry application types only:

~~~rust
pub struct QueryInput {
    pub site: SiteId,
    pub request: PostQueryRequest,
}

pub struct TagSuggestionInput {
    pub site: SiteId,
    pub request: TagSuggestionRequest,
}

pub struct RemoteCollectionInput {
    pub site: SiteId,
    pub collection: Option<RemoteCollectionRef>,
    pub pagination: PaginationRequest,
}

pub struct RemoteCollectionListInput {
    pub site: SiteId,
    pub pagination: PaginationRequest,
}

pub struct SavedQueryInput {
    pub id: Option<LocalRecordId>,
    pub site: SiteId,
    pub request: ReplayableQuery,
    pub name: String,
    pub pinned: bool,
    pub sort_order: Option<u32>,
}

pub struct AuthCompletion {
    pub challenge_id: String,
    pub completed: bool,
}

pub struct DownloadRequest {
    pub target: DownloadTarget,
}

pub struct EnqueueDownloadResult {
    pub record: DownloadRecord,
}
~~~

    list_sites() -> Vec<SiteDescriptor>
    get_site_capabilities(SiteId) -> EffectiveCapabilities
    get_site_config_schema(SiteId) -> SiteConfigSchema
    list_site_categories(SiteId) -> Vec<CategoryNode>
    list_site_mirrors(SiteId) -> Vec<MirrorDescriptor>
    load_site_config(SiteId) -> SiteConfig
    validate_site_config(SiteId, SiteConfigInput) -> SiteConfig
    save_site_config(SiteId, SiteConfigInput) -> SiteConfig
    load_config() -> RuntimeConfig
    save_config(RuntimeConfigInput) -> RuntimeConfig

    auth_status(SiteId) -> AuthStatus
    begin_auth(SiteId) -> AuthChallenge
    complete_auth(AuthCompletion) -> AuthStatus
    refresh_auth(SiteId) -> AuthStatus
    logout(SiteId) -> ()

    query_posts(QueryInput) -> PostPage
    continue_query(QuerySessionId) -> PostPage
    cancel_query(QuerySessionId) -> ()
    suggest_tags(TagSuggestionInput) -> Vec<TagSuggestion>
    lookup_post(PostRef) -> Post
    hydrate_post(PostRef, DetailExpansion) -> Post
    get_creator(CreatorRef) -> CreatorProfile
    list_remote_collections(RemoteCollectionListInput) -> RemoteCollectionPage
    list_remote_collection_posts(RemoteCollectionInput) -> PostPage
    list_remote_favorites(SiteId, PaginationRequest) -> PostPage
    save_query(SavedQueryInput) -> SavedQuery
    list_saved_queries(bool) -> Vec<SavedQuery>
    move_saved_query(LocalRecordId, Direction) -> ()
    delete_saved_query(LocalRecordId) -> ()
    download_history(PaginationRequest) -> Vec<DownloadRecord>
    open_download(LocalRecordId) -> ()
    execute_post_action(PostActionInput) -> PostActionResult

    enqueue_download(DownloadRequest) -> EnqueueDownloadResult
    cancel_download(LocalRecordId) -> ()
    retry_download(LocalRecordId) -> EnqueueDownloadResult
    open_site(SiteId) -> ()
    open_post(PostRef) -> ()

The current Tauri shell uses `open_site` for an explicit site-owned browser
route and for feed-error recovery. It validates the registered browse
capability first; the frontend cannot supply an arbitrary URL. For Konachan
this route is `https://konachan.com/post`, matching its feed/search transport
origin.
The post-detail action uses the same site-owned route boundary for
`https://konachan.com/post/show/<id>` and validates the site/post reference
before opening it.

Command rules:

- every remote command has a runtime operation ID and cancellation path;
- query_posts, list_remote_collections, and list_remote_collection_posts create
  a runtime-owned QuerySessionId when another page is available;
  continue_query is the only pagination command;
- the runtime browser integration completes BrowserSession challenges by
  importing cookies for the site's declared domains. complete_auth carries
  only a challenge ID and browser-flow result, never cookie material;
- optional commands return unsupported_capability when unavailable;
- auth-required commands return auth_required/auth_expired without secrets;
- DownloadTarget contains a PostRef/variant or site collection/archive
  format, never a URL/path/referer;
- enqueue_download returns a durable queue record immediately. Progress and
  terminal state are observed through runtime events and download_history;
  cancel_download and retry_download address the queue record, not a live
  frontend-held URL;
- remote collection commands are backed by RemoteCollectionCapability, not local
  rows or a frontend-supplied username.
- Collection archives are backed by CollectionDownloadCapability; the site
  resolves the archive URL and auth requirements while the runtime owns the
  local destination and download history.
- Download workers are asynchronous and resumable through local queue state;
  they never expose site URLs or temporary cache paths to React.

## Errors

~~~rust
pub struct RuntimeError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    pub operation_id: OperationId,
}

pub enum ErrorCode {
    SiteUnavailable,
    SiteDisabled,
    UnsupportedCapability,
    InvalidRequest,
    InvalidPagination,
    NotFound,
    Forbidden,
    AuthRequired,
    AuthExpired,
    RateLimited,
    RemoteStatus,
    DecodeFailed,
    NetworkFailed,
    Cancelled,
    ConfigInvalid,
    SecretStoreFailed,
    StorageFailed,
}
~~~

The frontend receives stable safe messages, never raw response bodies,
authorization headers, cookies, signed URLs, or local filesystem details.
Site diagnostics may be logged in Rust after redaction.

Mapping rules are loss-aware: site cancellation maps to Cancelled;
authentication failures to AuthRequired or AuthExpired; HTTP 401/403 to the
same auth/Forbidden family; 429 and site retry hints to RateLimited;
malformed payloads to DecodeFailed; transport failures to NetworkFailed; and
all other non-success statuses to RemoteStatus. The runtime always supplies
the operation_id and preserves retryable separately from the safe message.

## Runtime lifecycle and consistency

- A query session owns site ID, normalized request fingerprint, current
  pagination state, operation ID, cancellation, and seen PostRefs.
- Starting a new query cancels the previous operation. A late response is
  discarded even if HTTP completes successfully.
- Auth changes invalidate effective capabilities and continuation tokens bound
  to the previous session.
- Remote save mutation is serialized per (site, PostRef, collection). Its result is the
  authoritative remote-state update.
- Favorite-page refresh and mutation use request identity/version checks.
- Saved-query writes are idempotent by local record ID and do not depend on
  network availability.
- Download history is written transactionally as queued -> running -> terminal
  state; retries append attempts without losing the final outcome.
- Saved-query writes and remote favorite mutations are committed independently
  and are never represented as one undifferentiated boolean.
- Site pages may change while posts are inserted; v1 promises request-local
  deduplication, not a global snapshot.

## Yande v1 profile

| Capability | Yande behavior |
| --- | --- |
| PostQueryCapability | Search and `Feed(Popular { period, anchor_date })`; both use `/post.json`, while popular adds a date expression and `order:score`. |
| Pagination | Tag search and popular modes use page/limit; popular continuation stays inside the selected day/week/month window. |
| TagSuggestionCapability | Live Yande `/tag.json` with name/count/type/ambiguity metadata; the pinned MoeLoaderP adapter's `/tag.xml` behavior is source evidence, not the v1 endpoint contract. |
| PostLookupCapability | ID-filtered post query, subject to verified response behavior. |
| RemoteFavoriteCapability | Authenticated POST /post/vote.json; Yande maps favorite add/remove to score 3/2. |
| RemoteCollectionCapability | Searchable public pool metadata from `/pool.json?query=…` with ordered pool posts. |
| RemoteFavoriteListCapability | Not advertised until authorized current-user favorite-list semantics are verified. |
| CollectionDownloadCapability | Yande pool ZIP is exposed at `/pool/zip/:id`; the public pool page links it, but the observed request redirects anonymous users to login. |
| SiteAuth | Browser session at /user/login, user_info cookie detection with legacy user_id compatibility, secret-store-backed session. |
| Media | Preview, sample, JPEG/large, original, MD5, dimensions, rating, score, source, posting account, and timestamps where present. |

## Konachan v1 profile

| Capability | Konachan behavior |
| --- | --- |
| PostQueryCapability | Anonymous tag search and latest browse through the safe `.net` Moebooru JSON API. |
| Pagination | Page-numbered `post.json` requests with the requested limit. |
| TagSuggestionCapability | The same Moebooru tag JSON shape, normalized at the adapter boundary. |
| RelatedTagCapability | Explicit post-detail related-tag lookup through `/tag/related.json`; tuple counts are normalized and resulting searches still use the safe content policy. |
| PostLookupCapability | Exact numeric IDs are resolved with the safe `.net` API's `id:<post-id>` tag expression and verified against the returned post ID. |
| Browser post route | The detail action opens the site-owned `https://konachan.com/post/show/<id>` page; this is a browser route for bot-protected access, not a substitute API transport. |
| Similar search | The detail action opens the site-owned `https://konachan.com/post/similar` form; no direct API or query parameter is assumed. |
| RemoteCollectionCapability | Searchable public pool metadata from `/pool.json?query=…` with page/limit pagination, plus ordered safe-visible posts from `/pool/show.json?id=…`; the API may return the complete visible pool in one response. |
| Content policy | Safe only for the initial release; explicit-host access is not advertised because `.com` API requests are bot-protection sensitive. |
| Media | Preview, sample, full URL, dimensions, rating, score, author, source, checksum, and normalized timestamps where present. |
| Auth/favorites/pool ZIP | Not advertised in this slice; browser-session auth and remote favorites remain unverified, and Konachan does not expose a ZIP download capability. |

## Verification contract

Before API v1 approval, plan tests for:

1. site registration and capability consistency;
2. Yande legacy-array and v2 post envelopes;
3. tag encoding, suggestions, safe policy, empty pages, malformed responses,
   and rate-limit mapping;
4. popular day/week/month expression mapping, Monday-first week boundaries,
   page-2 continuation, short-page termination, and infinite-scroll UI;
5. missing metadata, rating/checksum mapping, scoped identity, and variants;
6. auth transitions without secret serialization;
7. favorite score 3/2 mapping, auth_required, idempotency, and race safety;
8. favorite-list current-user scoping, private visibility, ordering, empty
   state, and continuation;
9. pool metadata, ordered posts, ZIP archive capability, auth redirect, and
   archive download-history transitions, including site-specific listing
   page behavior;
10. discovery-source mapping for search, feeds, creators, site collections,
   and site-defined routes;
11. local SQLite queue durability, saved-query persistence, asynchronous
    staging/atomic-rename transitions, and `ExistingTarget` no-overwrite
    behavior;
12. command rejection of frontend URLs, paths, cookies, and site DTOs; and
13. cancellation, stale-result suppression, stable errors, and no-live-network
    unit tests.

Authorized live verification is separate from fixtures. Favorite mutation must
be reversible. Receipts record site SHA, endpoint family, timestamp, and
redacted statuses/counts, never credentials or raw private response data. The
PASS/BLOCKED rules are in YANDE-RELEASE-GATE.md.

## Review gaps before approval

These are deliberate review prompts:

- [ ] Is Post too broad for the wire model, or should PostSummary and
      PostDetail be separate?
- [ ] Should MediaVariant.url cross Tauri, or should previews/downloads use
      runtime-issued media handles?
- [ ] What TTL, memory limit, and restart behavior should apply to
      runtime-owned QuerySessionId handles?
- [ ] Should TagCategory and site tag counts be common fields or extensions?
- [ ] Should SortRequest use a typed common enum plus an extension escape hatch?
- [ ] Is FavoriteStatus::Unknown enough, or does the UI need a reason enum?
- [ ] What is Yande's authenticated favorite-list read contract, including
      current-user identity, private visibility, ordering, and pagination?
- [ ] Which site config fields are truly common?
- [ ] Are SiteDefined media variants acceptable in v1?
- [ ] Should lookup, detail, and collection remain separate traits?
- [ ] Should PostActionCapability ship in v1, or remain a later extension until
      a second real non-favorite action is required?
- [ ] Is TextWithMedia a sufficient social-site extension, or should
      threads/replies/quotes be a separate API family?
- [ ] What download-history retention rule should be persisted in SQLite v1?
- [ ] Should saved-query tabs and the remote favorite page be separate routes?
- [ ] What site-ID migration policy protects local rows when an adapter is
      renamed or removed?
- [ ] What exception policy applies if authorized live favorite-list verification
      cannot run in CI?

Until these questions are resolved, this is the review artifact and the Rust
scaffold remains intentionally smaller than the proposed contract.

# Dreamland API and runtime design v1

Status: draft for review. This document is a gate for implementation. No
provider, registry, or runtime trait should be implemented until this design is
approved.

## Objective

Define the stable boundary between the React/Tauri frontend and the Rust
runtime while keeping Dreamland provider-neutral. The v1 design must preserve
the existing browse, pagination, settings, and download behavior, then leave
room for search, favorites, tags, and batch downloads.

## Reference-informed direction

The reference projects validate several boundaries we should keep, while also
showing what Dreamland should improve:

- `yande` separates remote HTTP sources from a local DAO, persists favorites,
  tags, blocked tags, and download state, and keeps image filtering in the
  runtime layer;
- `MoeLoaderP` uses a site registry, per-site capability flags, cancellable
  search sessions, search history, multiple download URL variants, progress,
  retries, and bounded download concurrency;
- Dreamland should adopt those concepts without copying either project’s
  monolithic model or UI. Optional provider capabilities and separate local
  stores are the important lessons.

## Non-negotiable boundaries

- React owns rendering, interaction state, and view state only.
- Rust owns provider HTTP requests, response decoding, validation, config
  persistence, download I/O, and runtime errors.
- Tauri commands expose application concepts, not provider JSON or arbitrary
  filesystem/network primitives.
- `yandere` is the stable provider ID for the `yande.re` adapter. It is one
  provider adapter in the provider set, not the
  application-wide API contract.
- Provider credentials, if a future provider needs them, do not go into TOML;
  use a platform secret store when that requirement exists.

## Domain types

The exact Rust syntax may change during implementation, but the concepts and
ownership are fixed:

```rust
pub struct ProviderId(String);
pub struct PostId(String);

pub struct PostRef {
    pub provider: ProviderId,
    pub id: PostId,
}

pub struct ImagePost {
    pub reference: PostRef,
    pub tags: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub file_url: String,
    pub sample_url: String,
    pub preview_url: String,
    pub rating: String,
    pub score: Option<i32>,
    pub checksum: Option<String>,
    pub file_size: Option<u64>,
    pub download_variants: Vec<DownloadVariant>,
}

pub struct DownloadVariant {
    pub kind: DownloadVariantKind,
    pub url: String,
    pub referer: Option<String>,
    pub file_extension: Option<String>,
    pub file_size: Option<u64>,
}
```

`PostId` is a string because provider identifiers are not required to be
numeric. The `yandere` adapter converts its numeric IDs and MD5 values into the
provider-neutral representation. The frontend must never infer identity from
an MD5 or from a URL.

## Capabilities

Capabilities are declared by each provider and enforced by the runtime:

```rust
pub struct ProviderCapabilities {
    pub browse: bool,
    pub post_search: bool,
    pub tag_search: bool,
    pub tag_query: bool,
    pub post_lookup: bool,
    pub page_numbers: bool,
    pub cursors: bool,
    pub multiple_download_variants: bool,
}

pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub name: String,
    pub capabilities: ProviderCapabilities,
}
```

Search, tag, lookup, and download-variant controls are conditional features.
The frontend must not show a control that the selected provider cannot
satisfy. Favorites and local tags are Dreamland-owned data and are not
provider capabilities.

## Provider trait

The provider trait is the only runtime interface to an external image-board
API. The registry owns provider instances; the UI never selects a concrete
Rust type.

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn descriptor(&self) -> ProviderDescriptor;

    async fn browse(
        &self,
        request: BrowseRequest,
        cancel: CancellationToken,
    ) -> Result<PostPage, ProviderError>;
}

#[async_trait]
pub trait SearchProvider: Provider {
    async fn search_posts(
        &self,
        request: SearchRequest,
        cancel: CancellationToken,
    ) -> Result<PostPage, ProviderError>;
}

#[async_trait]
pub trait TagSearchProvider: Provider {
    async fn search_tags(
        &self,
        request: TagSearchRequest,
        cancel: CancellationToken,
    ) -> Result<Vec<Tag>, ProviderError>;
}

#[async_trait]
pub trait PostLookupProvider: Provider {
    async fn fetch_post(
        &self,
        reference: &PostRef,
        cancel: CancellationToken,
    ) -> Result<ImagePost, ProviderError>;
}
```

Optional capabilities are extension traits, not methods that every provider
must implement with a fake response. The registry reports the supported
capabilities and returns `unsupported_capability` when a selected provider
does not implement one.

The request types contain provider-neutral intent:

```rust
pub struct BrowseRequest {
    pub page: u32,
    pub per_page: u16,
}

pub struct SearchRequest {
    pub text: Option<String>,
    pub tags: Vec<String>,
    pub page: u32,
    pub per_page: u16,
}

pub struct TagSearchRequest {
    pub text: String,
    pub limit: u16,
}

pub struct Tag {
    pub name: String,
    pub kind: Option<String>,
    pub count: Option<u64>,
    pub ambiguous: Option<bool>,
}

pub struct PostPage {
    pub posts: Vec<ImagePost>,
    pub page: u32,
    pub has_next: bool,
}
```

The first implementation may use page numbers for the current desktop UI. A
cursor-based provider must be adapted inside the provider implementation; its
cursor must not leak into React in v1. If cursor support becomes necessary,
the contract changes here first.

Every provider request receives cancellation and request identity from the
runtime. A search session owns its provider, query, current page, and
cancellation state. A stale or cancelled response must not replace a newer
session’s results.

## Provider registry

```rust
pub trait ProviderRegistry: Send + Sync {
    fn list(&self) -> Vec<ProviderDescriptor>;
    fn get(&self, id: &ProviderId) -> Result<&dyn Provider, RuntimeError>;
}
```

The registry is constructed once by the Tauri runtime and is the single source
of provider instances. Adding a provider means adding an adapter and a
registration entry; it must not require frontend code that understands that
provider’s response shape.

## Runtime configuration

Runtime configuration is TOML and selects a provider without encoding provider
response data:

```toml
default_provider = "yandere"
download_path = "~/Downloads/dreamland_images"
images_per_page = 20

[providers.yandere]
base_url = "https://yande.re"
```

The implementation must read the new TOML format first and fall back to the
existing JSON file during the compatibility window. Tauri’s own
`tauri.conf.json` remains unrelated build configuration.

## Tauri command surface

The initial command surface is intentionally small:

```text
list_providers() -> ProviderDescriptor[]
load_config() -> RuntimeConfig
save_config(RuntimeConfigInput) -> RuntimeConfig
query_posts(QueryInput) -> PostPage
download_post(PostRef) -> DownloadResult
```

Future commands for favorites and batch downloads must use `PostRef`, not an
entire provider response object supplied by the frontend. Commands return
serializable application errors with a stable code and a user-facing message.

## Error model

The runtime distinguishes at least:

- `provider_unavailable` — provider is not registered or temporarily disabled;
- `unsupported_capability` — the provider cannot satisfy the requested mode;
- `invalid_request` — local validation failed;
- `remote_status` — provider returned an HTTP error;
- `decode_failed` — provider response did not match its adapter;
- `network_failed` — transport failed;
- `storage_failed` — local config or download I/O failed.

Provider-specific diagnostics stay in Rust logs/errors. The frontend receives
stable codes and safe messages, not raw response bodies or credentials.

## Testing contract

Before implementation is considered complete:

- a fake provider exercises the registry and capability checks;
- yande.re fixtures verify response decoding and normalization;
- command serialization tests verify the frontend-facing types;
- TOML load/save tests cover JSON fallback and invalid config;
- download tests use `PostRef` and never accept a frontend-provided path or
  filename as an authority;
- no test requires a live provider network request.

## Open decisions before implementation

1. Confirm the complete provider list and stable IDs.
2. Confirm whether v1 search uses free text, tags, or both per provider.
3. Choose storage for favorites and local tags; it is application data, not
   TOML configuration.
4. Confirm the first batch-download semantics: duplicate handling, retries,
   and cancellation.

import { invoke } from "@tauri-apps/api/core";

export type MediaVariant = "Preview" | "Sample" | "Full";

export type Rating = "Safe" | "Questionable" | "Explicit" | "Unknown";
export type ContentPolicy =
  | "SafeOnly"
  | "AllowQuestionable"
  | "AllowExplicit"
  | "ExplicitOnly";

export interface PostRef {
  site: string;
  id: string;
}

export interface AppConfig {
  download_path: string;
  images_per_page: number;
  content_policy: ContentPolicy;
  download_variant: MediaVariant;
  network: NetworkPolicy;
}

export type ProxyMode = "Auto" | "Direct" | { Manual: { url: string } };

export interface NetworkPolicy {
  proxy: ProxyMode;
  max_retries: number;
  retry_delay_ms: number;
  max_retry_delay_ms: number;
}

export interface SiteCapabilities {
  browse: boolean;
  post_search: boolean;
  tag_search: boolean;
  related_tags: boolean;
  tag_query: boolean;
  post_lookup: boolean;
  similar_search: boolean;
  page_numbers: boolean;
  cursors: boolean;
  multiple_download_variants: boolean;
  safe_content_only: boolean;
  authentication: boolean;
  remote_favorites: boolean;
  favorite_list: boolean;
  collections: boolean;
  collection_downloads: boolean;
}

export interface SiteDescriptor {
  id: string;
  name: string;
  capabilities: SiteCapabilities;
}

export interface ProxyDetection {
  detected: boolean;
  source: string | null;
  endpoint: string | null;
}

// Site-neutral post shape returned by every site adapter -- never a site's
// raw response JSON. Field names intentionally do not mirror any one site's
// wire format.
export interface Post {
  post: PostRef;
  tags: string[];
  author: string | null;
  creator_id: number | null;
  md5: string | null;
  source: string | null;
  parent_id: string | null;
  has_children: boolean;
  created_at: string | null;
  width: number | null;
  height: number | null;
  rating: Rating;
  score: number | null;
  preview_url: string | null;
  sample_url: string | null;
  full_url: string | null;
  file_size: number | null;
}

export type DiscoverySource =
  | "Browse"
  | { Search: { expression: string } }
  | { Feed: { kind: "Latest" | { Popular: { period: "Day" | "Week" | "Month"; anchor_date: string } } } };

export type PaginationRequest =
  | { First: { page_size: number } }
  | { Page: { number: number; page_size: number } }
  | "FixedWindow";

export interface PostQueryRequest {
  query: {
    source: DiscoverySource;
    content_policy: ContentPolicy;
  };
  pagination: PaginationRequest;
}

export interface SavedQuery {
  id: string;
  site: string;
  name: string;
  query: {
    source: DiscoverySource;
    content_policy: ContentPolicy;
  };
  pinned: boolean;
  position: number;
  updated_at_ms: number;
}

export interface SitePage {
  posts: Post[];
  continuation: "None" | { Next: string };
  total: number | null;
  page_size: number;
  session: string | null;
}

export interface Pool {
  site: string;
  id: string;
  name: string;
  post_count: number;
  public: boolean;
}

export interface PoolPage {
  pools: Pool[];
  page: number;
  page_size: number;
  has_next: boolean;
}

export interface ArchiveRecord {
  id: string;
  site: string;
  pool_id: string;
  pool_name: string;
  status: DownloadStatus;
  target_path: string | null;
  error: string | null;
  attempts: number;
  bytes_downloaded: number;
  total_bytes: number | null;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface AuthStatus {
  authenticated: boolean;
  username: string | null;
}

export interface TagSuggestion {
  name: string;
  category: string | null;
  post_count: number | null;
  ambiguous: boolean | null;
  aliases: string[];
}

export interface RelatedTag {
  name: string;
  post_count: number | null;
}

export type DownloadStatus =
  | "Queued"
  | "Running"
  | "Completed"
  | "Failed"
  | "Cancelled"
  | "ExistingTarget";

export interface DownloadRecord {
  id: string;
  site: string;
  post_id: string;
  variant: MediaVariant;
  status: DownloadStatus;
  target_path: string | null;
  error: string | null;
  attempts: number;
  bytes_downloaded: number;
  total_bytes: number | null;
  created_at_ms: number;
  updated_at_ms: number;
  metadata: Post;
}

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(
  downloadPath: string,
  contentPolicy: ContentPolicy,
  downloadVariant: MediaVariant,
  network: NetworkPolicy,
): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { downloadPath, contentPolicy, downloadVariant, network });
}

export function detectProxy(): Promise<ProxyDetection> {
  return invoke<ProxyDetection>("detect_proxy");
}

export function clearCache(): Promise<void> {
  return invoke<void>("clear_cache");
}

export function beginAuth(): Promise<void> {
  return invoke<void>("begin_auth");
}

export function openSite(siteId: string): Promise<void> {
  return invoke<void>("open_site", { siteId });
}

export function openPost(siteId: string, postId: string): Promise<void> {
  return invoke<void>("open_post", { siteId, postId });
}

export function openSimilarSearch(siteId: string): Promise<void> {
  return invoke<void>("open_similar_search", { siteId });
}

export function authStatus(): Promise<AuthStatus> {
  return invoke<AuthStatus>("auth_status");
}

export function signOut(): Promise<void> {
  return invoke<void>("sign_out");
}

export function listSites(): Promise<SiteDescriptor[]> {
  return invoke<SiteDescriptor[]>("list_sites");
}

export function listPools(siteId: string, query = "", page = 1, pageSize = 20): Promise<PoolPage> {
  return invoke<PoolPage>("list_pools", { siteId, query, page, pageSize });
}

export function queryPoolPosts(siteId: string, poolId: string, page = 1, pageSize = 20): Promise<SitePage> {
  return invoke<SitePage>("query_pool_posts", { siteId, poolId, page, pageSize });
}

export function enqueuePoolZip(poolId: string, poolName: string): Promise<ArchiveRecord> {
  return invoke<ArchiveRecord>("enqueue_pool_zip", { poolId, poolName });
}

export function setFavorite(postId: string, favorite: boolean): Promise<void> {
  return invoke<void>("set_favorite", { postId, favorite });
}

export function listFavorites(page = 1, pageSize = 20): Promise<SitePage> {
  return invoke<SitePage>("list_favorites", { page, pageSize });
}

export function listSavedQueries(siteId: string): Promise<SavedQuery[]> {
  return invoke<SavedQuery[]>("list_saved_queries", { siteId });
}

export function saveSavedQuery(saved: SavedQuery): Promise<SavedQuery> {
  return invoke<SavedQuery>("save_saved_query", { saved });
}

export function deleteSavedQuery(siteId: string, id: string): Promise<void> {
  return invoke<void>("delete_saved_query", { siteId, id });
}

export function moveSavedQuery(siteId: string, id: string, direction: -1 | 1): Promise<void> {
  return invoke<void>("move_saved_query", { siteId, id, direction });
}

export function queryDefaultPosts(page: number): Promise<Post[]> {
  return queryPosts("yandere", {
    query: { source: "Browse", content_policy: "SafeOnly" },
    pagination: { Page: { number: page, page_size: 20 } },
  }).then((result) => result.posts);
}

export function queryPosts(siteId: string, request: PostQueryRequest): Promise<SitePage> {
  return invoke<SitePage>("query_posts", {
    input: { site: siteId, request },
  });
}

export function lookupPost(siteId: string, postId: string, contentPolicy: ContentPolicy): Promise<Post> {
  return invoke<Post>("lookup_post", { siteId, postId, contentPolicy });
}

export function loadDetailImage(siteId: string, postId: string): Promise<string> {
  return invoke<string>("load_detail_image", { siteId, postId });
}

export function suggestTags(siteId: string, query: string, limit = 5): Promise<TagSuggestion[]> {
  return invoke<TagSuggestion[]>("suggest_tags", {
    input: {
      site: siteId,
      request: { query, limit },
    },
  });
}

export function relatedTags(siteId: string, tags: string[], limit = 12): Promise<RelatedTag[]> {
  return invoke<RelatedTag[]>("related_tags", { siteId, tags, limit });
}

export function continueQuery(session: string): Promise<SitePage> {
  return invoke<SitePage>("continue_query", { session });
}

export function cancelQuery(session: string): Promise<void> {
  return invoke<void>("cancel_query", { session });
}

export function enqueueDownload(siteId: string, postId: string, variant: MediaVariant): Promise<DownloadRecord> {
  return invoke<DownloadRecord>("enqueue_download", {
    siteId,
    postId,
    variant,
  });
}

export function cancelDownload(id: string): Promise<void> {
  return invoke<void>("cancel_download", { id });
}

export function retryDownload(id: string): Promise<DownloadRecord> {
  return invoke<DownloadRecord>("retry_download", { id });
}

export function listDownloads(limit = 50): Promise<DownloadRecord[]> {
  return invoke<DownloadRecord[]>("list_downloads", { limit });
}

export function listDownloadHistory(limit = 50, offset = 0): Promise<DownloadRecord[]> {
  return invoke<DownloadRecord[]>("list_download_history", { limit, offset });
}

export function listArchives(limit = 50): Promise<ArchiveRecord[]> {
  return invoke<ArchiveRecord[]>("list_archives", { limit });
}

export function cancelArchive(id: string): Promise<void> {
  return invoke<void>("cancel_archive", { id });
}

export function openDownload(path: string): Promise<void> {
  return invoke<void>("open_download", { path });
}

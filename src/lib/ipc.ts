import { invoke } from "@tauri-apps/api/core";

// Dreamland is a single-site app today (see docs/API-V1.md). The active
// site id is fixed here rather than exposed as a picker, since there is
// only one browse-capable site to pick.
const ACTIVE_SITE_ID = "yandere";

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
  api_url: string;
  images_per_page: number;
  network: NetworkPolicy;
}

export type ProxyMode = "Auto" | "Direct" | { Manual: { url: string } };

export interface NetworkPolicy {
  proxy: ProxyMode;
  max_retries: number;
  retry_delay_ms: number;
  max_retry_delay_ms: number;
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

export interface SitePage {
  posts: Post[];
  continuation: "None" | { Next: string };
  total: number | null;
  page_size: number;
  session: string | null;
}

export interface TagSuggestion {
  name: string;
  category: string | null;
  post_count: number | null;
  ambiguous: boolean | null;
  aliases: string[];
}

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(
  downloadPath: string,
  apiUrl: string,
  network: NetworkPolicy,
): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { downloadPath, apiUrl, network });
}

export function detectProxy(): Promise<ProxyDetection> {
  return invoke<ProxyDetection>("detect_proxy");
}

export function queryDefaultPosts(page: number): Promise<Post[]> {
  return queryPosts({
    query: { source: "Browse", content_policy: "SafeOnly" },
    pagination: { Page: { number: page, page_size: 20 } },
  }).then((result) => result.posts);
}

export function queryPosts(request: PostQueryRequest): Promise<SitePage> {
  return invoke<SitePage>("query_posts", {
    input: { site: ACTIVE_SITE_ID, request },
  });
}

export function suggestTags(query: string, limit = 5): Promise<TagSuggestion[]> {
  return invoke<TagSuggestion[]>("suggest_tags", {
    input: {
      site: ACTIVE_SITE_ID,
      request: { query, limit },
    },
  });
}

export function continueQuery(session: string): Promise<SitePage> {
  return invoke<SitePage>("continue_query", { session });
}

export function cancelQuery(session: string): Promise<void> {
  return invoke<void>("cancel_query", { session });
}

export function downloadImage(postId: string, variant: MediaVariant): Promise<string> {
  return invoke<string>("download_image", {
    siteId: ACTIVE_SITE_ID,
    postId,
    variant,
  });
}

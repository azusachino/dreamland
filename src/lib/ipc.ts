import { invoke } from "@tauri-apps/api/core";

// Dreamland is a single-site app today (see docs/API-V1.md). The active
// site id is fixed here rather than exposed as a picker, since there is
// only one browse-capable site to pick.
const ACTIVE_SITE_ID = "yandere";

export type MediaVariant = "Preview" | "Sample" | "Full";

export type Rating = "Safe" | "Questionable" | "Explicit" | "Unknown";

export interface PostRef {
  site: string;
  id: string;
}

export interface AppConfig {
  download_path: string;
  api_url: string;
  images_per_page: number;
}

// Site-neutral post shape returned by every site adapter -- never a site's
// raw response JSON. Field names intentionally do not mirror any one site's
// wire format.
export interface Post {
  post: PostRef;
  tags: string[];
  width: number;
  height: number;
  rating: Rating;
  score: number | null;
  preview_url: string;
  sample_url: string;
  full_url: string;
  file_size: number | null;
}

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(downloadPath: string, apiUrl: string): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { downloadPath, apiUrl });
}

export function loadImages(page: number): Promise<Post[]> {
  return invoke<Post[]>("load_images", { siteId: ACTIVE_SITE_ID, page });
}

export function downloadImage(postId: string, variant: MediaVariant): Promise<string> {
  return invoke<string>("download_image", {
    siteId: ACTIVE_SITE_ID,
    postId,
    variant,
  });
}

import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  download_path: string;
  api_url: string;
  images_per_page: number;
}

export interface ImagePost {
  id: number;
  tags: string;
  width: number;
  height: number;
  file_url: string;
  sample_url: string;
  preview_url: string;
  rating: string;
  score: number | null;
  md5: string;
  file_size: number | null;
}

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(downloadPath: string, apiUrl: string): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { downloadPath, apiUrl });
}

export function loadImages(page: number): Promise<ImagePost[]> {
  return invoke<ImagePost[]>("load_images", { page });
}

export function downloadImage(image: ImagePost): Promise<string> {
  return invoke<string>("download_image", { image });
}

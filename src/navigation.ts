import type { ViewMode } from "./view-model";

const viewPaths: Record<ViewMode, string> = {
  latest: "/latest",
  popular: "/popular",
  search: "/search",
  downloads: "/downloads",
  pools: "/pools",
  favorites: "/favorites",
  playground: "/playground",
};

interface ViewPathOptions {
  siteId?: string;
  tags?: string;
  period?: string;
  date?: string;
  query?: string;
}

export function viewFromPath(pathname: string): ViewMode {
  const path = pathname.replace(/\/+$/, "") || "/latest";
  if (path === "/pools" || path.startsWith("/pools/")) return "pools";
  const entry = Object.entries(viewPaths).find(([, value]) => value === path);
  return (entry?.[0] as ViewMode | undefined) ?? "latest";
}

export function pathForView(view: ViewMode, options: ViewPathOptions = {}): string {
  const params = new URLSearchParams();
  if (options.siteId) params.set("site", options.siteId);
  if (options.tags) params.set("tags", options.tags);
  if (options.period) params.set("period", options.period);
  if (options.date) params.set("date", options.date);
  if (options.query) params.set("query", options.query);
  const query = params.toString();
  return `${viewPaths[view]}${query ? `?${query}` : ""}`;
}

export function searchPath(expression: string, siteId?: string): string {
  return pathForView("search", { siteId, tags: expression });
}

export function popularPath(siteId: string, period: string, date: string): string {
  return pathForView("popular", { siteId, period, date });
}

export function poolPath(siteId: string, poolId?: string, query?: string): string {
  const path = poolId ? `/pools/${encodeURIComponent(poolId)}` : "/pools";
  const base = pathForView("pools", { siteId, query });
  return poolId ? `${path}${base.slice("/pools".length)}` : base;
}

export function isPoolPath(pathname: string): boolean {
  return /^\/pools\/[^/]+$/.test(pathname);
}

export function postPath(siteId: string, postId: string): string {
  return `/posts/${encodeURIComponent(siteId)}/${encodeURIComponent(postId)}`;
}

export function isPostPath(pathname: string): boolean {
  return /^\/posts\/[^/]+\/[^/]+$/.test(pathname);
}

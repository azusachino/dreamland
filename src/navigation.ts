import type { ViewMode } from "./view-model";

const viewPaths: Record<ViewMode, string> = {
  latest: "/latest",
  popular: "/popular",
  search: "/search",
  downloads: "/downloads",
  pools: "/pools",
  favorites: "/favorites",
};

export function viewFromPath(pathname: string): ViewMode {
  const path = pathname.replace(/\/+$/, "") || "/latest";
  const entry = Object.entries(viewPaths).find(([, value]) => value === path);
  return (entry?.[0] as ViewMode | undefined) ?? "latest";
}

export function pathForView(view: ViewMode): string {
  return viewPaths[view];
}

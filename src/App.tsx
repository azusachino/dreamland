import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  authStatus,
  beginAuth,
  cancelArchive,
  cancelQuery,
  continueQuery,
  detectProxy,
  enqueueDownload,
  enqueuePoolZip,
  listPools,
  listFavorites,
  listDownloads,
  listDownloadHistory,
  listArchives,
  listSavedQueries,
  lookupPost,
  cancelDownload,
  deleteSavedQuery,
  listSites,
  moveSavedQuery,
  openDownload,
  openPost,
  openSite,
  openSimilarSearch,
  relatedTags,
  retryDownload,
  loadConfig,
  queryPosts,
  queryPoolPosts,
  saveConfig,
  saveSavedQuery,
  setFavorite,
  signOut,
  suggestTags,
  type AppConfig,
  type ArchiveRecord,
  type AuthStatus,
  type ContentPolicy,
  type DiscoverySource,
  type DownloadRecord,
  type DownloadStatus,
  type MediaVariant,
  type NetworkPolicy,
  type Post,
  type PostQueryRequest,
  type Pool,
  type ProxyMode,
  type RelatedTag,
  type SavedQuery,
  type SiteDescriptor,
} from "./lib/ipc";

type ViewMode = "latest" | "popular" | "search" | "downloads" | "pools" | "favorites";
type PopularPeriod = "Day" | "Week" | "Month";
type ToastTone = "success" | "info" | "error";
type IconName = "clock" | "trend" | "download" | "book" | "heart" | "search" | "refresh" | "settings" | "close" | "back" | "forward" | "check" | "chevron";
type QueryOrder = "" | "score" | "score_asc" | "id" | "id_desc" | "mpixels" | "mpixels_asc" | "landscape" | "portrait" | "vote" | "random";

interface AdvancedQueryForm {
  tags: string;
  order: QueryOrder;
  rating: "" | "s" | "q" | "e" | "-s" | "-q" | "-e";
  score: string;
  scoreMin: string;
  scoreMax: string;
  idMin: string;
  idMax: string;
  voteMin: string;
  voteMax: string;
  mpixelsMin: string;
  mpixelsMax: string;
  widthMin: string;
  widthMax: string;
  heightMin: string;
  heightMax: string;
  dateFrom: string;
  dateTo: string;
  user: string;
  source: string;
  parent: string;
  pool: string;
  md5: string;
}

interface ToastState {
  id: number;
  title: string;
  message: string;
  tone: ToastTone;
}

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function today(): string {
  const date = new Date();
  return [date.getFullYear(), date.getMonth() + 1, date.getDate()]
    .map((part, index) => index === 0 ? String(part).padStart(4, "0") : String(part).padStart(2, "0"))
    .join("-");
}

function parseCalendarDate(value: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const date = new Date(`${value}T00:00:00Z`);
  return date.toISOString().slice(0, 10) === value ? date : null;
}

function formatCalendarDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function normalizePopularAnchor(anchor: string, period: PopularPeriod): string {
  const date = parseCalendarDate(anchor) ?? parseCalendarDate(today())!;
  if (period === "Week") {
    date.setUTCDate(date.getUTCDate() - ((date.getUTCDay() + 6) % 7));
  } else if (period === "Month") {
    date.setUTCDate(1);
  }
  return formatCalendarDate(date);
}

function popularWindow(anchor: string, period: PopularPeriod): [string, string] {
  const start = parseCalendarDate(normalizePopularAnchor(anchor, period))!;
  const end = new Date(start);
  if (period === "Day") {
    return [formatCalendarDate(start), formatCalendarDate(start)];
  }
  if (period === "Week") {
    end.setUTCDate(end.getUTCDate() + 6);
  } else {
    end.setUTCMonth(end.getUTCMonth() + 1, 0);
  }
  return [formatCalendarDate(start), formatCalendarDate(end)];
}

function shiftPopularAnchor(anchor: string, period: PopularPeriod, direction: -1 | 1): string {
  const date = parseCalendarDate(normalizePopularAnchor(anchor, period))!;
  if (period === "Month") {
    date.setUTCMonth(date.getUTCMonth() + direction);
  } else {
    date.setUTCDate(date.getUTCDate() + direction * (period === "Week" ? 7 : 1));
  }
  return formatCalendarDate(date);
}

function savedQueryExpression(saved: SavedQuery): string | null {
  const source = saved.query.source;
  return typeof source === "object" && "Search" in source ? source.Search.expression : null;
}

function savedQueryDescription(saved: SavedQuery): string {
  const source = saved.query.source;
  if (source === "Browse") return "latest posts";
  if ("Search" in source) return source.Search.expression;
  if (source.Feed.kind === "Latest") return "latest feed";
  return `${source.Feed.kind.Popular.period.toLowerCase()} popular feed`;
}

function savedQueryIsRunnable(saved: SavedQuery): boolean {
  return savedQueryExpression(saved) !== null;
}

function addRangeTerm(terms: string[], name: string, exact: string, minimum: string, maximum: string) {
  const value = exact.trim();
  const min = minimum.trim();
  const max = maximum.trim();
  if (value) terms.push(`${name}:${value}`);
  else if (min && max) terms.push(`${name}:${min}..${max}`);
  else if (min) terms.push(`${name}:${min}..`);
  else if (max) terms.push(`${name}:..${max}`);
}

function buildAdvancedQuery(form: AdvancedQueryForm, contentPolicy: ContentPolicy): string {
  const terms = form.tags.trim().split(/\s+/).filter(Boolean);
  if (form.order) terms.push(`order:${form.order}`);
  const allowedRatings: Record<ContentPolicy, AdvancedQueryForm["rating"][]> = {
    SafeOnly: ["", "s"],
    AllowQuestionable: ["", "s", "q", "-e"],
    AllowExplicit: ["", "s", "q", "e", "-s", "-q", "-e"],
    ExplicitOnly: ["", "e"],
  };
  if (!allowedRatings[contentPolicy].includes(form.rating)) {
    throw new Error(`Rating filter conflicts with the ${contentPolicyLabel(contentPolicy).toLowerCase()} content policy`);
  }
  if (form.rating) terms.push(form.rating.startsWith("-") ? `-rating:${form.rating.slice(1)}` : `rating:${form.rating}`);
  for (const [name, value] of [["score", form.score], ["score minimum", form.scoreMin], ["score maximum", form.scoreMax], ["post ID minimum", form.idMin], ["post ID maximum", form.idMax], ["vote minimum", form.voteMin], ["vote maximum", form.voteMax], ["width minimum", form.widthMin], ["width maximum", form.widthMax], ["height minimum", form.heightMin], ["height maximum", form.heightMax]] as const) {
    if (value.trim() && !/^-?\d+$/.test(value.trim())) throw new Error(`${name} must be an integer`);
  }
  for (const [name, value] of [["megapixels minimum", form.mpixelsMin], ["megapixels maximum", form.mpixelsMax]] as const) {
    if (value.trim() && !/^\d+(\.\d+)?$/.test(value.trim())) throw new Error(`${name} must be a non-negative number`);
  }
  for (const [name, value] of [["post ID minimum", form.idMin], ["post ID maximum", form.idMax], ["vote minimum", form.voteMin], ["vote maximum", form.voteMax], ["width minimum", form.widthMin], ["width maximum", form.widthMax], ["height minimum", form.heightMin], ["height maximum", form.heightMax]] as const) {
    if (value.trim() && Number(value) < 0) throw new Error(`${name} must not be negative`);
  }
  for (const [name, minimum, maximum] of [["score", form.scoreMin, form.scoreMax], ["post ID", form.idMin, form.idMax], ["votes", form.voteMin, form.voteMax], ["megapixels", form.mpixelsMin, form.mpixelsMax], ["width", form.widthMin, form.widthMax], ["height", form.heightMin, form.heightMax]] as const) {
    if (minimum && maximum && Number(minimum) > Number(maximum)) throw new Error(`${name} minimum must not exceed maximum`);
  }
  if (form.score && (form.scoreMin || form.scoreMax)) throw new Error("Use exact score or a score range, not both");
  if (form.dateFrom && !parseCalendarDate(form.dateFrom)) throw new Error("Date from must use a valid calendar date");
  if (form.dateTo && !parseCalendarDate(form.dateTo)) throw new Error("Date to must use a valid calendar date");
  if (form.dateFrom && form.dateTo && form.dateFrom > form.dateTo) throw new Error("Date from must not be later than date to");
  addRangeTerm(terms, "score", form.score, form.scoreMin, form.scoreMax);
  addRangeTerm(terms, "id", "", form.idMin, form.idMax);
  addRangeTerm(terms, "vote", "", form.voteMin, form.voteMax);
  addRangeTerm(terms, "mpixels", "", form.mpixelsMin, form.mpixelsMax);
  addRangeTerm(terms, "width", "", form.widthMin, form.widthMax);
  addRangeTerm(terms, "height", "", form.heightMin, form.heightMax);
  if (form.dateFrom && form.dateTo) terms.push(`date:${form.dateFrom}..${form.dateTo}`);
  else if (form.dateFrom) terms.push(`date:${form.dateFrom}..`);
  else if (form.dateTo) terms.push(`date:..${form.dateTo}`);
  for (const [name, value] of [["user", form.user], ["source", form.source], ["parent", form.parent], ["pool", form.pool], ["md5", form.md5]] as const) {
    if (value.trim()) terms.push(`${name}:${value.trim()}`);
  }
  if (!terms.length) throw new Error("Add a tag or filter before searching");
  return terms.join(" ");
}

function emptyAdvancedQueryForm(tags = ""): AdvancedQueryForm {
  return {
    tags,
    order: "",
    rating: "",
    score: "",
    scoreMin: "",
    scoreMax: "",
    idMin: "",
    idMax: "",
    voteMin: "",
    voteMax: "",
    mpixelsMin: "",
    mpixelsMax: "",
    widthMin: "",
    widthMax: "",
    heightMin: "",
    heightMax: "",
    dateFrom: "",
    dateTo: "",
    user: "",
    source: "",
    parent: "",
    pool: "",
    md5: "",
  };
}

function parseRange(value: string): [string, string, string] {
  const range = value.match(/^([^.]*)\.\.([^.]*)$/);
  if (!range) return [value, "", ""];
  return ["", range[1] ?? "", range[2] ?? ""];
}

function parseAdvancedQuery(expression: string): AdvancedQueryForm {
  const form = emptyAdvancedQueryForm();
  const rawTags: string[] = [];
  for (const token of expression.trim().split(/\s+/).filter(Boolean)) {
    const match = token.match(/^(-?)([a-z_]+):(.+)$/i);
    if (!match) {
      rawTags.push(token);
      continue;
    }
    const [, prefix, name, value] = match;
    const field = name.toLowerCase();
    if (prefix && !["rating"].includes(field)) {
      rawTags.push(token);
      continue;
    }
    if (field === "order" && !prefix && ["score", "score_asc", "id", "id_desc", "mpixels", "mpixels_asc", "landscape", "portrait", "vote", "random"].includes(value as QueryOrder)) {
      form.order = value as QueryOrder;
    } else if (field === "rating" && ["s", "q", "e"].includes(value)) {
      form.rating = prefix ? `-${value}` as AdvancedQueryForm["rating"] : value as AdvancedQueryForm["rating"];
    } else if (["score", "id", "vote", "mpixels", "width", "height"].includes(field)) {
      const [exact, minimum, maximum] = parseRange(value);
      if (field === "score") [form.score, form.scoreMin, form.scoreMax] = [exact, minimum, maximum];
      if (field === "id") [form.idMin, form.idMax] = [minimum || exact, maximum];
      if (field === "vote") [form.voteMin, form.voteMax] = [minimum || exact, maximum];
      if (field === "mpixels") [form.mpixelsMin, form.mpixelsMax] = [minimum || exact, maximum];
      if (field === "width") [form.widthMin, form.widthMax] = [minimum || exact, maximum];
      if (field === "height") [form.heightMin, form.heightMax] = [minimum || exact, maximum];
    } else if (field === "date") {
      const [, minimum, maximum] = parseRange(value);
      if (minimum || maximum) [form.dateFrom, form.dateTo] = [minimum, maximum];
      else rawTags.push(token);
    } else if (["user", "source", "parent", "pool", "md5"].includes(field)) {
      const identityField = field as "user" | "source" | "parent" | "pool" | "md5";
      form[identityField] = value;
    } else {
      rawTags.push(token);
    }
  }
  form.tags = rawTags.join(" ");
  return form;
}

function contentPolicyLabel(policy: ContentPolicy): string {
  switch (policy) {
    case "SafeOnly": return "safe only";
    case "AllowQuestionable": return "safe + questionable";
    case "AllowExplicit": return "all ratings";
    case "ExplicitOnly": return "explicit only";
  }
}

function isActiveDownload(status: DownloadStatus): boolean {
  return status === "Queued" || status === "Running";
}

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    clock: "M12 6v6l4 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
    trend: "m4 16 5-5 4 3 7-8M15 6h5v5",
    download: "M12 3v12m0 0 5-5m-5 5-5-5M4 21h16",
    book: "M4 5.5A2.5 2.5 0 0 1 6.5 3H11v17H6.5A2.5 2.5 0 0 0 4 22V5.5Zm16 0A2.5 2.5 0 0 0 17.5 3H13v17h4.5A2.5 2.5 0 0 1 20 22V5.5Z",
    heart: "m12 20-7-7a4.5 4.5 0 0 1 6.4-6.3L12 8.3l.6-1.6A4.5 4.5 0 0 1 19 13z",
    search: "m21 21-4.4-4.4m2.4-5.1a7.5 7.5 0 1 1-15 0 7.5 7.5 0 0 1 15 0Z",
    refresh: "M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35Z",
    settings: "M19.43 12.98c.04-.32.07-.65.07-.98s-.02-.66-.07-.98l2.11-1.65c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.37-.31-.6-.22l-2.49 1c-.52-.4-1.08-.73-1.69-.98L14.5 2.42C14.46 2.18 14.25 2 14 2h-4c-.25 0-.46.18-.5.42L9.12 5.07c-.61.25-1.17.58-1.69.98l-2.49-1c-.23-.08-.48 0-.6.22l-2 3.46c-.12.22-.07.49.12.64l2.11 1.65c-.04.32-.08.65-.08.98s.03.66.08.98l-2.11 1.65c-.19.15-.24.42-.12.64l2 3.46c.12.22.37.31.6.22l2.49-1c.52.4 1.08.73 1.69.98l.38 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.38-2.65c.61-.25 1.17-.58 1.69-.98l2.49 1c.23.08.48 0 .6-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.65ZM12 15.5A3.5 3.5 0 1 1 12 8a3.5 3.5 0 0 1 0 7.5Z",
    close: "M6 6l12 12M18 6 6 18",
    back: "M19 12H5m6 6-6-6 6-6",
    forward: "M5 12h14m-6-6 6 6-6 6",
    check: "m5 12 4 4L19 6",
    chevron: "m6 9 6 6 6-6",
  };
  const material = name === "refresh" || name === "settings";
  return <svg className="icon" viewBox="0 0 24 24" fill={material ? "currentColor" : "none"} stroke={material ? "none" : "currentColor"} strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}

function App() {
  const queryClient = useQueryClient();
  const [view, setView] = useState<ViewMode>("latest");
  const [selectedSiteId, setSelectedSiteId] = useState(() => window.localStorage.getItem("dreamland.site") ?? "yandere");
  const [popularPeriod, setPopularPeriod] = useState<PopularPeriod>("Week");
  const [popularAnchorDate, setPopularAnchorDate] = useState(() => normalizePopularAnchor(today(), "Week"));
  const [selectedSavedQueryId, setSelectedSavedQueryId] = useState<string | null>(null);
  const [editingSavedQueryId, setEditingSavedQueryId] = useState<string | null>(null);
  const [searchDraft, setSearchDraft] = useState("");
  const [submittedSearch, setSubmittedSearch] = useState("");
  const [searchFocused, setSearchFocused] = useState(false);
  const [siteMenuOpen, setSiteMenuOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [advancedSearchOpen, setAdvancedSearchOpen] = useState(false);
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [poolSearchDraft, setPoolSearchDraft] = useState("");
  const [submittedPoolSearch, setSubmittedPoolSearch] = useState("");
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const [relatedTagsOpen, setRelatedTagsOpen] = useState(false);
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedPostIds, setSelectedPostIds] = useState<Set<string>>(new Set());
  const [favoritePostIds, setFavoritePostIds] = useState<Set<string>>(new Set());
  const [batchDownloading, setBatchDownloading] = useState(false);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [toast, setToast] = useState<ToastState | null>(null);
  const toastId = useRef(0);
  const searchInput = useRef<HTMLInputElement>(null);
  const sitePicker = useRef<HTMLDivElement>(null);
  const downloadStatuses = useRef(new Map<string, DownloadStatus>());
  const activeQuerySession = useRef<string | null>(null);

  function showToast(title: string, message: string, tone: ToastTone = "success") {
    const id = ++toastId.current;
    setToast({ id, title, message, tone });
    window.setTimeout(() => {
      setToast((current) => current?.id === id ? null : current);
    }, 4_500);
  }

  const configQuery = useQuery({
    queryKey: ["config"],
    queryFn: loadConfig,
  });
  const sitesQuery = useQuery({
    queryKey: ["sites"],
    queryFn: listSites,
  });
  const activeSite = sitesQuery.data?.find((site) => site.id === selectedSiteId && site.capabilities.browse)
    ?? sitesQuery.data?.find((site) => site.capabilities.browse)
    ?? null;
  const activeSiteId = activeSite?.id ?? "yandere";
  const activeSiteSafeOnly = activeSite?.capabilities.safe_content_only ?? false;
  const authQuery = useQuery({
    queryKey: ["auth"],
    queryFn: authStatus,
    enabled: activeSite?.capabilities.authentication === true,
  });
  const savedQueriesQuery = useQuery({
    queryKey: ["saved-queries", activeSiteId],
    queryFn: () => listSavedQueries(activeSiteId),
    enabled: configQuery.isSuccess && Boolean(activeSite),
  });
  const pageSize = configQuery.data?.images_per_page ?? 20;
  const contentPolicy = activeSiteSafeOnly ? "SafeOnly" : configQuery.data?.content_policy ?? "SafeOnly";
  const activeSavedQuery = savedQueriesQuery.data?.find((saved) => saved.id === selectedSavedQueryId);
  const activeContentPolicy = activeSiteSafeOnly
    ? "SafeOnly"
    : activeSavedQuery?.query.content_policy ?? contentPolicy;
  const request = useMemo<PostQueryRequest>(() => {
    let source: DiscoverySource = "Browse";
    if (view === "popular") {
      source = {
        Feed: {
          kind: {
            Popular: { period: popularPeriod, anchor_date: popularAnchorDate },
          },
        },
      };
    } else if (view === "search" && submittedSearch) {
      source = { Search: { expression: submittedSearch } };
    }

    return {
      query: { source, content_policy: activeContentPolicy },
      pagination: { First: { page_size: pageSize } },
    };
  }, [activeContentPolicy, pageSize, popularAnchorDate, popularPeriod, submittedSearch, view]);
  const imagesQuery = useInfiniteQuery({
    queryKey: ["posts", activeSiteId, request],
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) => pageParam ? continueQuery(pageParam) : queryPosts(activeSiteId, request),
    getNextPageParam: (lastPage) => lastPage.session && lastPage.posts.length >= lastPage.page_size
      ? lastPage.session
      : undefined,
    enabled: configQuery.isSuccess && sitesQuery.isSuccess && Boolean(activeSite) && (view === "latest" || view === "popular" || view === "search"),
  });
  useEffect(() => {
    activeQuerySession.current = imagesQuery.data?.pages.at(-1)?.session ?? null;
  }, [imagesQuery.data]);
  useEffect(() => {
    return () => {
      const session = activeQuerySession.current;
      if (session) void cancelQuery(session).catch(() => undefined);
    };
  }, [activeSiteId, request]);
  const downloadsQuery = useInfiniteQuery({
    queryKey: ["downloads"],
    initialPageParam: 0,
    queryFn: ({ pageParam }) => pageParam === 0 ? listDownloads(50) : listDownloadHistory(50, pageParam),
    getNextPageParam: (lastPage, pages) => {
      const historyCount = lastPage.filter((record) => !isActiveDownload(record.status)).length;
      return historyCount === 50 ? pages.length * 50 : undefined;
    },
    enabled: configQuery.isSuccess,
    refetchInterval: 1_500,
  });
  const archivesQuery = useQuery({
    queryKey: ["archives"],
    queryFn: () => listArchives(50),
    enabled: configQuery.isSuccess,
    refetchInterval: 1_500,
  });
  useEffect(() => {
    const records = downloadsQuery.data?.pages.flatMap((page) => page) ?? [];
    if (!records.length) return;
    for (const record of records) {
      const previous = downloadStatuses.current.get(record.id);
      if (previous && previous !== record.status) {
        if (record.status === "Completed") {
          showToast("download complete", `post #${record.post_id} is ready in downloads.`);
        } else if (record.status === "ExistingTarget") {
          showToast("already downloaded", `post #${record.post_id} was not overwritten.`, "info");
        } else if (record.status === "Failed") {
          showToast("download failed", record.error ?? `post #${record.post_id} could not be saved.`, "error");
        }
      }
      downloadStatuses.current.set(record.id, record.status);
    }
  }, [downloadsQuery.data]);
  const suggestionsQuery = useQuery({
    queryKey: ["tag-suggestions", activeSiteId, searchDraft.trim()],
    queryFn: () => suggestTags(activeSiteId, searchDraft.trim(), 7),
    enabled: searchFocused && searchDraft.trim().length >= 1 && activeSite?.capabilities.tag_search === true,
    staleTime: 30_000,
  });
  const poolsQuery = useInfiniteQuery({
    queryKey: ["pools", activeSiteId, submittedPoolSearch],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => listPools(activeSiteId, submittedPoolSearch, pageParam, 30),
    getNextPageParam: (lastPage, pages) => lastPage.has_next ? pages.length + 1 : undefined,
    enabled: view === "pools" && selectedPool === null && activeSite?.capabilities.collections === true,
  });
  const poolPostsQuery = useInfiniteQuery({
    queryKey: ["pool-posts", selectedPool?.site, selectedPool?.id, contentPolicy],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => queryPoolPosts(activeSiteId, selectedPool!.id, pageParam, pageSize),
    getNextPageParam: (lastPage, pages) => {
      if (lastPage.total !== null) {
        const loaded = pages.reduce((count, page) => count + page.posts.length, 0);
        return loaded < lastPage.total ? pages.length + 1 : undefined;
      }
      return lastPage.posts.length >= lastPage.page_size ? pages.length + 1 : undefined;
    },
    enabled: view === "pools" && selectedPool !== null && configQuery.isSuccess && activeSite?.capabilities.collections === true,
  });
  const postDetailQuery = useQuery({
    queryKey: ["post-detail", activeSiteId, selectedPost?.post.id],
    queryFn: () => lookupPost(activeSiteId, selectedPost!.post.id, activeContentPolicy),
    enabled: Boolean(selectedPost) && activeSite?.capabilities.post_lookup === true,
    staleTime: 60_000,
  });
  const relatedTagsQuery = useQuery({
    queryKey: ["related-tags", activeSiteId, selectedPost?.post.id, selectedPost?.tags],
    queryFn: () => relatedTags(activeSiteId, selectedPost!.tags, 12),
    enabled: relatedTagsOpen && Boolean(selectedPost) && activeSite?.capabilities.related_tags === true,
    staleTime: 300_000,
  });
  const favoritesQuery = useInfiniteQuery({
    queryKey: ["favorites"],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => listFavorites(pageParam, 20),
    getNextPageParam: (lastPage, pages) => lastPage.posts.length >= lastPage.page_size ? pages.length + 1 : undefined,
    enabled: view === "favorites" && activeSite?.capabilities.favorite_list === true && authQuery.data?.authenticated === true && Boolean(authQuery.data.username),
  });
  useEffect(() => {
    const posts = favoritesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
    if (!posts.length) return;
    setFavoritePostIds((current) => new Set([...current, ...posts.map((post) => post.post.id)]));
  }, [favoritesQuery.data]);
  const saveConfigMutation = useMutation({
    mutationFn: ({ downloadPath, contentPolicy, downloadVariant, network }: ConfigInput) =>
      saveConfig(downloadPath, contentPolicy, downloadVariant, network),
    onSuccess: (nextConfig) => {
      queryClient.setQueryData(["config"], nextConfig);
      setSettingsOpen(false);
      setNotice("settings saved");
      showToast("settings saved", "your preferences are now active.");
    },
    onError: (reason) => setError(`failed to save settings: ${errorMessage(reason)}`),
  });
  const downloadMutation = useMutation({
    mutationFn: ({ siteId, postId, variant }: DownloadInput) => enqueueDownload(siteId, postId, variant),
    onSuccess: (record) => {
      void queryClient.invalidateQueries({ queryKey: ["downloads"] });
      downloadStatuses.current.set(record.id, record.status);
      setNotice(`added post #${record.post_id} to the download queue`);
      showToast("download queued", `post #${record.post_id} will be saved at the configured path.`);
    },
    onError: (reason) => setError(`download failed: ${errorMessage(reason)}`),
  });

  const isBrowseView = view === "latest" || view === "popular" || view === "search";
  const queryError = sitesQuery.error
    ? `failed to load sites: ${errorMessage(sitesQuery.error)}`
    : configQuery.error
      ? `failed to load configuration: ${errorMessage(configQuery.error)}`
      : isBrowseView && imagesQuery.error
        ? `failed to load images: ${errorMessage(imagesQuery.error)}`
        : "";
  const images = imagesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const favoritePosts = favoritesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const pools = poolsQuery.data?.pages.flatMap((page) => page.pools).filter((pool) => pool.public) ?? [];
  const poolPosts = poolPostsQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const selectedPosts = images.filter((post) => selectedPostIds.has(post.post.id));
  const previewPosts = useMemo(
    () => view === "pools" ? poolPosts : view === "favorites" ? favoritePosts : images,
    [favoritePosts, images, poolPosts, view],
  );
  const selectedPreviewIndex = selectedPost
    ? previewPosts.findIndex((post) => post.post.site === selectedPost.post.site && post.post.id === selectedPost.post.id)
    : -1;
  const canGoPrevious = selectedPreviewIndex > 0;
  const canGoNext = selectedPreviewIndex >= 0 && selectedPreviewIndex < previewPosts.length - 1;
  const loading = configQuery.isPending || sitesQuery.isPending || imagesQuery.isPending;
  const loadingMore = imagesQuery.isFetchingNextPage;
  const downloadRecords = useMemo(() => {
    const records = downloadsQuery.data?.pages.flatMap((page) => page) ?? [];
    return [...new Map(records.map((record) => [record.id, record])).values()];
  }, [downloadsQuery.data]);
  const title = view === "search"
    ? "search results"
    : view === "popular"
      ? "popular"
      : view === "downloads"
      ? "downloads"
      : view === "pools"
      ? "pools"
      : view === "favorites"
      ? "favorites"
      : "latest posts";
  const subtitle = view === "search"
    ? `matching “${submittedSearch}”`
    : view === "popular"
      ? `most popular this ${popularPeriod.toLowerCase()} · ${popularWindow(popularAnchorDate, popularPeriod).join(" to ")} · score-ranked`
      : view === "downloads"
      ? "local download history and active work"
      : view === "pools"
      ? `ordered public collections from ${activeSite?.name ?? "the active site"}`
      : view === "favorites"
      ? authQuery.data?.authenticated ? `saved by ${authQuery.data.username ?? "your yandere account"}` : "sign in to browse your saved posts"
      : activeSiteSafeOnly
      ? "safe-mode browse from the active site"
      : "a calm feed for finding something worth keeping";

  useEffect(() => {
    const site = sitesQuery.data?.find((candidate) => candidate.id === selectedSiteId);
    if (!site || !site.capabilities.browse) {
      const fallback = sitesQuery.data?.find((candidate) => candidate.capabilities.browse);
      if (fallback) {
        window.localStorage.setItem("dreamland.site", fallback.id);
        setSelectedSiteId(fallback.id);
      }
    }
  }, [selectedSiteId, sitesQuery.data]);

  useEffect(() => {
    if (!selectedPost) return;
    function handlePreviewKey(event: KeyboardEvent) {
      const target = event.target;
      if (target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA", "BUTTON"].includes(target.tagName)) return;
      if (event.key === "Escape") {
        event.preventDefault();
        setSelectedPost(null);
      } else if (event.key === "ArrowLeft" && canGoPrevious) {
        event.preventDefault();
        setSelectedPost(previewPosts[selectedPreviewIndex - 1] ?? null);
      } else if (event.key === "ArrowRight" && canGoNext) {
        event.preventDefault();
        setSelectedPost(previewPosts[selectedPreviewIndex + 1] ?? null);
      }
    }
    window.addEventListener("keydown", handlePreviewKey);
    return () => window.removeEventListener("keydown", handlePreviewKey);
  }, [activeContentPolicy, canGoNext, canGoPrevious, previewPosts, selectedPost, selectedPreviewIndex]);

  useEffect(() => {
    if (!siteMenuOpen) return;
    function closeSiteMenu(event: PointerEvent) {
      if (event.target instanceof Node && !sitePicker.current?.contains(event.target)) {
        setSiteMenuOpen(false);
      }
    }
    document.addEventListener("pointerdown", closeSiteMenu);
    return () => document.removeEventListener("pointerdown", closeSiteMenu);
  }, [siteMenuOpen]);

  function selectSite(site: SiteDescriptor) {
    if (!site.capabilities.browse || site.id === activeSiteId) return;
    window.localStorage.setItem("dreamland.site", site.id);
    setSelectedSiteId(site.id);
    setSiteMenuOpen(false);
    setError("");
    setNotice("");
    setView("latest");
    setSelectedSavedQueryId(null);
    setEditingSavedQueryId(null);
    setSelectedPool(null);
    setPoolSearchDraft("");
    setSubmittedPoolSearch("");
    setSelectedPost(null);
    setRelatedTagsOpen(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function submitPoolSearch(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setSelectedPool(null);
    setSubmittedPoolSearch(poolSearchDraft.trim());
  }

  function clearPoolSearch() {
    setPoolSearchDraft("");
    setSubmittedPoolSearch("");
  }

  function selectAdjacentPost(offset: -1 | 1) {
    const nextIndex = selectedPreviewIndex + offset;
    const nextPost = previewPosts[nextIndex];
    if (nextPost) setSelectedPost(nextPost);
  }

  function changeView(nextView: ViewMode) {
    setError("");
    setNotice("");
    setView(nextView);
    setSelectedSavedQueryId(null);
    setEditingSavedQueryId(null);
    setSelectedPool(null);
    setSelectedPost(null);
    setRelatedTagsOpen(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function submitSearch(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    const expression = searchDraft.trim();
    if (!expression) {
      changeView("latest");
      return;
    }
    setError("");
    setNotice("");
    setSubmittedSearch(expression);
    setSelectedSavedQueryId(null);
    setEditingSavedQueryId(null);
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function chooseTag(tag: string) {
    setSearchDraft(tag);
    setSubmittedSearch(tag);
    setSelectedSavedQueryId(null);
    setEditingSavedQueryId(null);
    setView("search");
    setSearchFocused(false);
    setSelectedPost(null);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function applyAdvancedQuery(expression: string) {
    setSearchDraft(expression);
    setSubmittedSearch(expression);
    setSelectedSavedQueryId(editingSavedQueryId);
    setEditingSavedQueryId(null);
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
    setAdvancedSearchOpen(false);
  }

  function openSavedQuery(saved: SavedQuery) {
    const expression = savedQueryExpression(saved);
    if (!expression) return;
    setSelectedSavedQueryId(saved.id);
    setEditingSavedQueryId(null);
    setSearchDraft(expression);
    setSubmittedSearch(expression);
    setView("search");
    setError("");
    setNotice("");
  }

  function editSavedQuery(saved: SavedQuery) {
    const expression = savedQueryExpression(saved);
    if (!expression) return;
    setSelectedSavedQueryId(saved.id);
    setEditingSavedQueryId(saved.id);
    setSearchDraft(expression);
    setSubmittedSearch(expression);
    setView("search");
    setAdvancedSearchOpen(true);
    setError("");
    setNotice("");
  }

  async function handleSaveQuery() {
    if (view !== "search" || !submittedSearch) return;
    const name = window.prompt("name this saved query", activeSavedQuery?.name ?? submittedSearch.trim());
    if (!name?.trim()) return;
    try {
      const saved = await saveSavedQuery({
        id: activeSavedQuery?.id ?? "",
        site: activeSiteId,
        name: name.trim(),
        query: {
          source: { Search: { expression: submittedSearch } },
          content_policy: activeContentPolicy,
        },
        pinned: false,
        position: savedQueriesQuery.data?.length ?? 0,
        updated_at_ms: 0,
      });
      await savedQueriesQuery.refetch();
      setSelectedSavedQueryId(saved.id);
      setNotice(`saved query “${saved.name}”`);
    } catch (reason) {
      setError(`could not save query: ${errorMessage(reason)}`);
    }
  }

  async function handleToggleSavedPin(saved: SavedQuery) {
    try {
      await saveSavedQuery({ ...saved, pinned: !saved.pinned });
      await savedQueriesQuery.refetch();
    } catch (reason) {
      setError(`could not update saved query: ${errorMessage(reason)}`);
    }
  }

  async function handleDeleteSavedQuery(saved: SavedQuery) {
    if (!window.confirm(`delete “${saved.name}”?`)) return;
    try {
      await deleteSavedQuery(activeSiteId, saved.id);
      await savedQueriesQuery.refetch();
      if (selectedSavedQueryId === saved.id) {
        setSelectedSavedQueryId(null);
        setView("latest");
      }
    } catch (reason) {
      setError(`could not delete query: ${errorMessage(reason)}`);
    }
  }

  async function handleMoveSavedQuery(saved: SavedQuery, direction: -1 | 1) {
    try {
      await moveSavedQuery(activeSiteId, saved.id, direction);
      await savedQueriesQuery.refetch();
    } catch (reason) {
      setError(`could not reorder saved query: ${errorMessage(reason)}`);
    }
  }

  function togglePostSelection(postId: string) {
    setSelectedPostIds((current) => {
      const next = new Set(current);
      if (next.has(postId)) next.delete(postId);
      else next.add(postId);
      return next;
    });
  }

  function selectCurrentPage() {
    setSelectedPostIds(new Set(images.map((post) => post.post.id)));
  }

  async function handleBatchDownload() {
    if (selectedPosts.length === 0) return;
    setBatchDownloading(true);
    setError("");
    setNotice("");
    let queued = 0;
    for (const post of selectedPosts) {
      try {
        await downloadMutation.mutateAsync({ siteId: post.post.site, postId: post.post.id, variant: configQuery.data?.download_variant ?? "Full" });
        queued += 1;
      } catch {
        // Each failed item is reported by the mutation; continue the batch.
      }
    }
    setBatchDownloading(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
    if (queued > 0) showToast("batch queued", `${queued} post${queued === 1 ? "" : "s"} added to downloads.`);
  }

  async function handleFavorite(post: Post) {
    if (!authQuery.data?.authenticated) {
      setView("favorites");
      showToast("sign in required", "connect your yandere account before changing favorites.", "info");
      return;
    }
    if (!activeSite?.capabilities.remote_favorites) {
      setError(`${activeSite?.name ?? "this site"} does not support remote favorites.`);
      return;
    }
    const favorite = !favoritePostIds.has(post.post.id);
    try {
      await setFavorite(post.post.id, favorite);
      void queryClient.invalidateQueries({ queryKey: ["favorites"] });
      setFavoritePostIds((current) => {
        const next = new Set(current);
        if (favorite) next.add(post.post.id);
        else next.delete(post.post.id);
        return next;
      });
      showToast(favorite ? "added to favorites" : "removed from favorites", `post #${post.post.id} updated on yandere.`);
    } catch (reason) {
      setError(`favorite failed: ${errorMessage(reason)}`);
    }
  }

  async function handlePoolZip(pool: Pool) {
    setError("");
    try {
      await enqueuePoolZip(pool.id, pool.name);
      await archivesQuery.refetch();
      showToast("pool zip queued", `${pool.name} will be saved in downloads.`);
    } catch (reason) {
      setError(`pool zip failed: ${errorMessage(reason)}`);
    }
  }

  async function handleDownload(post: Post) {
    setDownloadingId(post.post.id);
    setError("");
    setNotice("");
    try {
      await downloadMutation.mutateAsync({ siteId: post.post.site, postId: post.post.id, variant: configQuery.data?.download_variant ?? "Full" });
    } catch {
      // The mutation reports the user-facing error.
    } finally {
      setDownloadingId(null);
    }
  }

  async function handleSaveConfig(
    downloadPath: string,
    contentPolicy: ContentPolicy,
    downloadVariant: MediaVariant,
    network: NetworkPolicy,
  ) {
    setError("");
    setNotice("");
    try {
      await saveConfigMutation.mutateAsync({ downloadPath, contentPolicy, downloadVariant, network });
    } catch {
      // The mutation reports the user-facing error.
    }
  }

  async function handleOpenSite() {
    setError("");
    try {
      await openSite(activeSiteId);
    } catch (reason) {
      setError(`could not open site: ${errorMessage(reason)}`);
    }
  }

  async function handleOpenPost(post: Post) {
    setError("");
    try {
      await openPost(post.post.site, post.post.id);
    } catch (reason) {
      setError(`could not open ${activeSite?.name ?? "site"} post: ${errorMessage(reason)}`);
    }
  }

  async function handleOpenSimilarSearch() {
    setError("");
    try {
      await openSimilarSearch(activeSiteId);
    } catch (reason) {
      setError(`could not open ${activeSite?.name ?? "site"} similar search: ${errorMessage(reason)}`);
    }
  }

  return (
    <div className="app">
      <header className="app-header shell-surface">
        <div className="header-identity">
          <div className="brand-lockup">
            <div className="brand-mark" aria-hidden="true">✦</div>
            <div>
              <p className="eyebrow">image board</p>
              <h1>Dreamland</h1>
            </div>
          </div>
          <div className="site-picker" ref={sitePicker}>
            <span>browse site</span>
            <div className="site-picker-controls">
              <div className="site-menu">
                <button
                  className="site-menu-trigger"
                  type="button"
                  aria-label="choose image board site"
                  aria-expanded={siteMenuOpen}
                  onClick={() => setSiteMenuOpen((current) => !current)}
                >
                  <span>{activeSite?.name ?? "choose site"}</span>
                  <Icon name="chevron" />
                </button>
                {siteMenuOpen && <div className="site-menu-popover" role="menu">
                {(sitesQuery.data ?? []).map((site) => (
                  <button
                    key={site.id}
                    className={`site-menu-option${site.id === activeSiteId ? " active" : ""}`}
                    type="button"
                    role="menuitem"
                    disabled={!site.capabilities.browse}
                    onClick={() => selectSite(site)}
                    title={site.capabilities.browse ? `browse ${site.name}` : `${site.name} coming later`}
                  >
                    <span>{site.name}</span>
                    <small>{site.capabilities.browse ? "ready" : "later"}</small>
                  </button>
                ))}
                </div>}
              </div>
              <button className="button button-text site-open" type="button" onClick={() => void handleOpenSite()} disabled={!activeSite}>
                open site
              </button>
            </div>
          </div>
        </div>
        <form className="search-bar" onSubmit={submitSearch} role="search">
          <span className="search-icon"><Icon name="search" /></span>
          <input
            ref={searchInput}
            aria-label="search tags"
            placeholder="Search by tags…"
            value={searchDraft}
            onFocus={() => setSearchFocused(true)}
            onBlur={() => window.setTimeout(() => setSearchFocused(false), 120)}
            onChange={(event) => setSearchDraft(event.target.value)}
          />
          {searchDraft && (
            <button
              className="clear-search"
              type="button"
              aria-label="clear search"
              onClick={() => {
                setSearchDraft("");
                changeView("latest");
              }}
            >
              ×
            </button>
          )}
          <button className="search-advanced" type="button" onClick={() => {
            setEditingSavedQueryId(null);
            setAdvancedSearchOpen(true);
          }}>filters</button>
          {searchFocused && suggestionsQuery.data && suggestionsQuery.data.length > 0 && (
            <div className="suggestions" role="listbox">
              <p className="suggestion-heading">suggested tags</p>
              {suggestionsQuery.data.map((tag) => (
                <button
                  key={tag.name}
                  className="suggestion"
                  type="button"
                  role="option"
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => chooseTag(tag.name)}
                >
                  <span>{tag.name}</span>
                  {tag.post_count !== null && <small>{tag.post_count.toLocaleString()}</small>}
                </button>
              ))}
            </div>
          )}
        </form>
        <div className="header-actions">
          <button
            className="icon-button"
            type="button"
            aria-label={`refresh ${title.toLowerCase()}`}
            title={`refresh ${title.toLowerCase()}`}
            disabled={loading}
            onClick={() => {
              if (isBrowseView) void imagesQuery.refetch();
              else if (view === "favorites") void favoritesQuery.refetch();
              else if (view === "pools") void poolsQuery.refetch();
              else void downloadsQuery.refetch();
            }}
          >
            <Icon name="refresh" />
          </button>
          <button className="button button-tonal button-with-icon" type="button" onClick={() => setSettingsOpen(true)}>
            <Icon name="settings" />
            <span>settings</span>
          </button>
        </div>
      </header>

      <div className={`app-layout${selectedPost ? " has-detail" : ""}`}>
        <main className="content">
          <nav className="view-tabs shell-surface" aria-label="Dreamland sections" role="tablist">
            <NavButton active={view === "latest"} label="latest" icon="clock" onClick={() => changeView("latest")} />
            <NavButton active={view === "popular"} label="popular" icon="trend" onClick={() => changeView("popular")} />
          {savedQueriesQuery.data?.map((saved, index, savedQueries) => {
            const previous = savedQueries[index - 1];
            const next = savedQueries[index + 1];
            const canMoveUp = Boolean(previous && previous.pinned === saved.pinned);
            const canMoveDown = Boolean(next && next.pinned === saved.pinned);
            return (
            <div className="saved-query-nav" key={saved.id}>
              <button
                className={`nav-item${selectedSavedQueryId === saved.id ? " active" : ""}`}
                type="button"
                role="tab"
                aria-selected={selectedSavedQueryId === saved.id}
                disabled={!savedQueryIsRunnable(saved)}
                onClick={() => openSavedQuery(saved)}
                title={savedQueryIsRunnable(saved) ? savedQueryDescription(saved) : `${savedQueryDescription(saved)} (not supported yet)`}
              >
                <Icon name="search" />
                <span>{saved.name}</span>
                {!savedQueryIsRunnable(saved) && <small>unavailable</small>}
                {saved.pinned && savedQueryIsRunnable(saved) && <small>pinned</small>}
              </button>
              <div className="saved-query-actions">
                <button className="icon-button" type="button" aria-label={`edit ${saved.name}`} title="edit query" disabled={!savedQueryIsRunnable(saved)} onClick={() => editSavedQuery(saved)}>✎</button>
                <button className="icon-button" type="button" aria-label={`move ${saved.name} earlier`} title="move earlier" disabled={!canMoveUp} onClick={() => void handleMoveSavedQuery(saved, -1)}>↑</button>
                <button className="icon-button" type="button" aria-label={`move ${saved.name} later`} title="move later" disabled={!canMoveDown} onClick={() => void handleMoveSavedQuery(saved, 1)}>↓</button>
                <button className="icon-button" type="button" aria-label={`${saved.pinned ? "unpin" : "pin"} ${saved.name}`} onClick={() => void handleToggleSavedPin(saved)}>
                  {saved.pinned ? "•" : "○"}
                </button>
                <button className="icon-button" type="button" aria-label={`delete ${saved.name}`} onClick={() => void handleDeleteSavedQuery(saved)}>×</button>
              </div>
            </div>
            );
          })}
          {activeSite?.capabilities.collections && (
            <NavButton active={view === "pools"} label="pools" icon="book" onClick={() => changeView("pools")} />
          )}
          {activeSite?.capabilities.favorite_list && (
            <NavButton active={view === "favorites"} label="favorites" icon="heart" onClick={() => changeView("favorites")} />
          )}
          <NavButton active={view === "downloads"} label="downloads" icon="download" onClick={() => changeView("downloads")} />
          <span className="view-status">
            <span className="connection-dot" aria-hidden="true" />
            <span>{activeSiteSafeOnly ? `${activeSite?.name ?? "site"} · safe mode` : `${activeSite?.name ?? "site"} connected`}</span>
          </span>
          </nav>
          <section className="content-heading">
            <div>
              <p className="eyebrow">explore freely</p>
              <h2>{title}</h2>
              <p className="subtitle">{subtitle}</p>
            </div>
            <div className="heading-actions">
              {isBrowseView && <>
              <button className="button button-outlined" type="button" onClick={() => {
                setSelectionMode((current) => !current);
                setSelectedPostIds(new Set());
              }}>
                {selectionMode ? "cancel selection" : "select posts"}
              </button>
              {view === "popular" && (
                <div className="popular-controls" aria-label="popular period and date">
                  <label className="period-picker">
                    <span>period</span>
                    <select value={popularPeriod} onChange={(event) => {
                      const nextPeriod = event.target.value as PopularPeriod;
                      setPopularPeriod(nextPeriod);
                      setPopularAnchorDate((current) => normalizePopularAnchor(current, nextPeriod));
                    }}>
                      <option value="Day">day</option>
                      <option value="Week">week</option>
                      <option value="Month">month</option>
                    </select>
                  </label>
                  <button
                    className="button button-outlined"
                    type="button"
                    aria-label={`Earlier popular ${popularPeriod.toLowerCase()}`}
                    onClick={() => setPopularAnchorDate(shiftPopularAnchor(popularAnchorDate, popularPeriod, -1))}
                  >
                    earlier
                  </button>
                  <label className="period-picker period-date">
                    <span>date</span>
                    <input
                      type="date"
                      value={popularAnchorDate}
                      max={today()}
                      onChange={(event) => {
                        if (event.target.value && event.target.value <= today()) {
                          setPopularAnchorDate(normalizePopularAnchor(event.target.value, popularPeriod));
                        }
                      }}
                    />
                  </label>
                  <button
                    className="button button-outlined"
                    type="button"
                    aria-label={`Later popular ${popularPeriod.toLowerCase()}`}
                    disabled={shiftPopularAnchor(popularAnchorDate, popularPeriod, 1) > normalizePopularAnchor(today(), popularPeriod)}
                    onClick={() => setPopularAnchorDate(shiftPopularAnchor(popularAnchorDate, popularPeriod, 1))}
                  >
                    later
                  </button>
                </div>
              )}
              {view === "search" && submittedSearch && (
                <button className="button button-outlined" type="button" onClick={() => void handleSaveQuery()}>save query</button>
              )}
              <span className="content-policy">{contentPolicyLabel(activeContentPolicy)}</span>
              </>}
            </div>
          </section>

          {isBrowseView && selectionMode && (
            <div className="batch-toolbar" role="toolbar" aria-label="Batch download">
              <strong>{selectedPosts.length} selected</strong>
              <button className="button button-text" type="button" onClick={selectCurrentPage}>select loaded posts</button>
              <button className="button button-primary" type="button" disabled={!selectedPosts.length || batchDownloading} onClick={() => void handleBatchDownload()}>
                {batchDownloading ? "queueing…" : "download selected"}
              </button>
            </div>
          )}

          {error && !queryError && <p className="message message-error" role="alert">{error}</p>}
          {notice && <p className="message message-success" role="status">{notice}</p>}
          {isBrowseView ? (
            <>
          {loading && <div className="loading-line" role="status"><span /> finding something good…</div>}
              {queryError ? (
                <ErrorState
                  message={queryError}
                  onRetry={() => void imagesQuery.refetch()}
                  onOpenSite={activeSite ? () => void openSite(activeSiteId).catch((reason) => setError(`could not open site: ${errorMessage(reason)}`)) : undefined}
                  siteName={activeSite?.name}
                />
              ) : !loading && images.length === 0 ? (
                <div className="empty-state">
                  <span className="empty-symbol" aria-hidden="true">✦</span>
                  <h3>no posts found</h3>
                  <p>try a broader tag search or switch back to the latest feed.</p>
                </div>
              ) : (
                <>
                  <div className="gallery-grid">
                    {images.map((post) => (
                      <ImageCard
                        key={post.post.id}
                        post={post}
                        selectionMode={selectionMode}
                        selected={selectedPostIds.has(post.post.id)}
                        downloading={downloadingId === post.post.id}
                        onDownload={handleDownload}
                        onSelect={setSelectedPost}
                        onToggleSelection={() => togglePostSelection(post.post.id)}
                        onTag={chooseTag}
                      />
                    ))}
                  </div>
                  <LoadMore
                    hasNext={Boolean(imagesQuery.hasNextPage)}
                    loading={loadingMore}
                    onLoadMore={() => void imagesQuery.fetchNextPage()}
                  />
                </>
              )}
            </>
          ) : view === "downloads" ? (
            <DownloadPanel
              records={downloadRecords}
              archives={archivesQuery.data ?? []}
              historyHasNext={Boolean(downloadsQuery.hasNextPage)}
              historyLoading={downloadsQuery.isFetchingNextPage}
              onCancel={async (id) => {
                await cancelDownload(id);
                await queryClient.invalidateQueries({ queryKey: ["downloads"] });
              }}
              onRetry={async (id) => {
                await retryDownload(id);
                await queryClient.invalidateQueries({ queryKey: ["downloads"] });
              }}
              onOpen={async (path) => {
                try {
                  await openDownload(path);
                } catch (reason) {
                  setError(`could not open file: ${errorMessage(reason)}`);
                }
              }}
              onLoadMore={() => void downloadsQuery.fetchNextPage()}
              onCancelArchive={async (id) => {
                await cancelArchive(id);
                await archivesQuery.refetch();
              }}
            />
          ) : view === "pools" ? (
            <PoolPanel
              pools={pools}
              poolsLoading={poolsQuery.isPending || poolsQuery.isFetching}
              poolsError={poolsQuery.error ? errorMessage(poolsQuery.error) : ""}
              poolsHasNext={Boolean(poolsQuery.hasNextPage)}
              selectedPool={selectedPool}
              posts={poolPosts}
              postsLoading={poolPostsQuery.isPending || poolPostsQuery.isFetching}
              postsError={poolPostsQuery.error ? errorMessage(poolPostsQuery.error) : ""}
              postsHasNext={Boolean(poolPostsQuery.hasNextPage)}
              onRetryPools={() => void poolsQuery.refetch()}
              onRetryPosts={() => void poolPostsQuery.refetch()}
              onLoadMorePools={() => void poolsQuery.fetchNextPage()}
              onLoadMorePosts={() => void poolPostsQuery.fetchNextPage()}
              onBack={() => setSelectedPool(null)}
              onBrowse={setSelectedPool}
              onDownloadZip={(pool) => void handlePoolZip(pool)}
              onSelectPost={setSelectedPost}
              onDownload={handleDownload}
              onTag={chooseTag}
              downloadingId={downloadingId}
              siteName={activeSite?.name ?? "site"}
              collectionDownloads={activeSite?.capabilities.collection_downloads === true}
              poolSearch={poolSearchDraft}
              onPoolSearchChange={setPoolSearchDraft}
              onSubmitPoolSearch={submitPoolSearch}
              onClearPoolSearch={clearPoolSearch}
            />
          ) : (
            <AccountPanel
              auth={authQuery.data ?? null}
              favorites={favoritePosts}
              favoritesLoading={favoritesQuery.isPending || favoritesQuery.isFetching}
              favoritesError={favoritesQuery.error ? errorMessage(favoritesQuery.error) : ""}
              favoritesHasNext={Boolean(favoritesQuery.hasNextPage)}
              loading={authQuery.isFetching}
              onBeginAuth={() => void beginAuth()}
              onRefresh={() => void authQuery.refetch()}
              onRetry={() => void favoritesQuery.refetch()}
              onLoadMore={() => void favoritesQuery.fetchNextPage()}
              onSelect={setSelectedPost}
              onDownload={handleDownload}
              onTag={chooseTag}
              onSignOut={async () => {
                await signOut();
                await authQuery.refetch();
                setFavoritePostIds(new Set());
              }}
            />
          )}
        </main>

        {selectedPost && (
          <PostInspector
            post={postDetailQuery.data ?? selectedPost}
            detailLoading={postDetailQuery.isFetching}
            detailError={postDetailQuery.error ? errorMessage(postDetailQuery.error) : ""}
            downloading={downloadingId === selectedPost.post.id}
            siteName={activeSite?.name ?? selectedPost.post.site}
            favoriteSupported={activeSite?.capabilities.remote_favorites === true}
            onClose={() => setSelectedPost(null)}
            canGoPrevious={canGoPrevious}
            canGoNext={canGoNext}
            previewPosition={selectedPreviewIndex >= 0 ? selectedPreviewIndex + 1 : undefined}
            previewTotal={previewPosts.length}
            onPrevious={() => selectAdjacentPost(-1)}
            onNext={() => selectAdjacentPost(1)}
            onOpenPost={handleOpenPost}
            onOpenSimilarSearch={handleOpenSimilarSearch}
            similarSearchSupported={activeSite?.capabilities.similar_search === true}
            onDownload={handleDownload}
            favorited={favoritePostIds.has(selectedPost.post.id)}
            onFavorite={handleFavorite}
            onTag={chooseTag}
            relatedTagsSupported={activeSite?.capabilities.related_tags === true}
            relatedTagsOpen={relatedTagsOpen}
            relatedTags={relatedTagsQuery.data ?? []}
            relatedTagsLoading={relatedTagsQuery.isPending || relatedTagsQuery.isFetching}
            relatedTagsError={relatedTagsQuery.error ? errorMessage(relatedTagsQuery.error) : ""}
            onToggleRelatedTags={() => setRelatedTagsOpen((current) => !current)}
            downloadVariant={configQuery.data?.download_variant ?? "Full"}
          />
        )}
      </div>

      {settingsOpen && (
        <SettingsDialog
          config={configQuery.data ?? null}
          onCancel={() => setSettingsOpen(false)}
          onSave={handleSaveConfig}
        />
      )}
      {advancedSearchOpen && (
        <AdvancedQueryDialog
          contentPolicy={contentPolicy}
          initialExpression={submittedSearch || searchDraft}
          onClose={() => setAdvancedSearchOpen(false)}
          onApply={applyAdvancedQuery}
        />
      )}
      {toast && <Toast state={toast} onClose={() => setToast(null)} />}
    </div>
  );
}

interface ConfigInput {
  downloadPath: string;
  contentPolicy: ContentPolicy;
  downloadVariant: MediaVariant;
  network: NetworkPolicy;
}

interface DownloadInput {
  siteId: string;
  postId: string;
  variant: MediaVariant;
}

interface NavButtonProps {
  active?: boolean;
  disabled?: boolean;
  hint?: string;
  icon: IconName;
  label: string;
  onClick?: () => void;
}

function NavButton({ active, disabled, hint, icon, label, onClick }: NavButtonProps) {
  return (
    <button className={`nav-item${active ? " active" : ""}`} disabled={disabled} onClick={onClick} title={hint} role="tab" aria-selected={active}>
      <span className="nav-icon"><Icon name={icon} /></span>
      <span>{label}</span>
      {hint && <small>{hint}</small>}
    </button>
  );
}

interface AdvancedQueryDialogProps {
  contentPolicy: ContentPolicy;
  initialExpression: string;
  onClose: () => void;
  onApply: (expression: string) => void;
}

function AdvancedQueryDialog({ contentPolicy, initialExpression, onClose, onApply }: AdvancedQueryDialogProps) {
  const [form, setForm] = useState<AdvancedQueryForm>(() => parseAdvancedQuery(initialExpression));
  const [error, setError] = useState("");

  function update<K extends keyof AdvancedQueryForm>(key: K, value: AdvancedQueryForm[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      onApply(buildAdvancedQuery(form, contentPolicy));
    } catch (reason) {
      setError(errorMessage(reason));
    }
  }

  return (
    <div className="settings-overlay" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <form className="settings-dialog shell-surface query-dialog" onSubmit={submit} role="dialog" aria-modal="true" aria-labelledby="query-title">
        <div className="inspector-heading">
          <div><p className="eyebrow">moebooru query vocabulary</p><h2 id="query-title">advanced search</h2></div>
          <button className="icon-button" type="button" aria-label="close advanced search" onClick={onClose}><Icon name="close" /></button>
        </div>
        <p className="helper-text">build or edit a reusable tag query with the same filters supported by the vendored moebooru client. unknown terms stay in the raw tag field.</p>
        {error && <div className="panel-error" role="alert"><p>{error}</p></div>}
        <div className="settings-section">
          <p className="section-label">tags and ordering</p>
          <label className="field">tags or raw terms<input value={form.tags} onChange={(event) => update("tags", event.target.value)} placeholder="artist_name -sketch" autoFocus /></label>
          <div className="field-grid">
            <label className="field">Order
              <select value={form.order} onChange={(event) => update("order", event.target.value as QueryOrder)}>
                <option value="">site default</option>
                <option value="score">highest score</option>
                <option value="score_asc">lowest score</option>
                <option value="id_desc">newest id</option>
                <option value="id">oldest id</option>
                <option value="mpixels">largest pixels</option>
                <option value="mpixels_asc">smallest pixels</option>
                <option value="landscape">landscape</option>
                <option value="portrait">portrait</option>
                <option value="vote">most votes</option>
                <option value="random">random</option>
              </select>
            </label>
            <label className="field">rating
              <select value={form.rating} onChange={(event) => update("rating", event.target.value as AdvancedQueryForm["rating"])}>
                <option value="">policy default</option>
                <option value="s">safe</option>
                <option value="q">questionable</option>
                <option value="e">explicit</option>
                <option value="-s">exclude safe</option>
                <option value="-q">exclude questionable</option>
                <option value="-e">exclude explicit</option>
              </select>
            </label>
          </div>
        </div>
        <div className="settings-section">
          <p className="section-label">numeric ranges</p>
          <label className="field">exact score<input type="number" value={form.score} onChange={(event) => update("score", event.target.value)} placeholder="e.g. 10" /></label>
          <div className="field-grid">
            <label className="field">minimum width<input type="number" min="0" value={form.widthMin} onChange={(event) => update("widthMin", event.target.value)} /></label>
            <label className="field">maximum width<input type="number" min="0" value={form.widthMax} onChange={(event) => update("widthMax", event.target.value)} /></label>
            <label className="field">minimum height<input type="number" min="0" value={form.heightMin} onChange={(event) => update("heightMin", event.target.value)} /></label>
            <label className="field">maximum height<input type="number" min="0" value={form.heightMax} onChange={(event) => update("heightMax", event.target.value)} /></label>
          </div>
          <div className="field-grid">
            <label className="field">minimum score<input type="number" value={form.scoreMin} onChange={(event) => update("scoreMin", event.target.value)} /></label>
            <label className="field">maximum score<input type="number" value={form.scoreMax} onChange={(event) => update("scoreMax", event.target.value)} /></label>
            <label className="field">minimum post id<input type="number" min="0" value={form.idMin} onChange={(event) => update("idMin", event.target.value)} /></label>
            <label className="field">maximum post id<input type="number" min="0" value={form.idMax} onChange={(event) => update("idMax", event.target.value)} /></label>
            <label className="field">minimum votes<input type="number" min="0" value={form.voteMin} onChange={(event) => update("voteMin", event.target.value)} /></label>
            <label className="field">maximum votes<input type="number" min="0" value={form.voteMax} onChange={(event) => update("voteMax", event.target.value)} /></label>
            <label className="field">minimum megapixels<input type="number" min="0" step="0.1" value={form.mpixelsMin} onChange={(event) => update("mpixelsMin", event.target.value)} /></label>
            <label className="field">maximum megapixels<input type="number" min="0" step="0.1" value={form.mpixelsMax} onChange={(event) => update("mpixelsMax", event.target.value)} /></label>
          </div>
        </div>
        <div className="settings-section">
          <p className="section-label">date and identity</p>
          <div className="field-grid">
            <label className="field">date from<input type="date" value={form.dateFrom} max={today()} onChange={(event) => update("dateFrom", event.target.value)} /></label>
            <label className="field">date to<input type="date" value={form.dateTo} max={today()} onChange={(event) => update("dateTo", event.target.value)} /></label>
            <label className="field">user / author tag<input value={form.user} onChange={(event) => update("user", event.target.value)} placeholder="user name" /></label>
            <label className="field">source<input value={form.source} onChange={(event) => update("source", event.target.value)} /></label>
            <label className="field">parent id<input inputMode="numeric" value={form.parent} onChange={(event) => update("parent", event.target.value)} /></label>
            <label className="field">pool id<input inputMode="numeric" value={form.pool} onChange={(event) => update("pool", event.target.value)} /></label>
          </div>
          <label className="field">md5 checksum<input value={form.md5} onChange={(event) => update("md5", event.target.value)} /></label>
        </div>
        <div className="dialog-actions">
          <button className="button button-text" type="button" onClick={onClose}>cancel</button>
          <button className="button button-primary" type="submit">search</button>
        </div>
      </form>
    </div>
  );
}

interface ErrorStateProps {
  message: string;
  onRetry: () => void;
  onOpenSite?: () => void;
  siteName?: string;
}

function ErrorState({ message, onRetry, onOpenSite, siteName }: ErrorStateProps) {
  return (
    <div className="error-state" role="alert">
      <span className="error-symbol" aria-hidden="true">!</span>
      <div>
        <h3>couldn’t load this feed</h3>
        <p>{message.replace("Failed to load images: ", "")}</p>
      </div>
      <button className="button button-outlined" type="button" onClick={onRetry}>try again</button>
      {onOpenSite && <button className="button button-outlined" type="button" onClick={onOpenSite}>open {siteName}</button>}
    </div>
  );
}

interface ToastProps {
  state: ToastState;
  onClose: () => void;
}

function Toast({ state, onClose }: ToastProps) {
  return (
    <aside className={`toast toast-${state.tone}`} role="status" aria-live="polite">
      <div>
        <strong>{state.title}</strong>
        <p>{state.message}</p>
      </div>
      <button className="icon-button" type="button" aria-label="dismiss notification" onClick={onClose}><Icon name="close" /></button>
    </aside>
  );
}

interface LoadMoreProps {
  autoLoad?: boolean;
  hasNext: boolean;
  loading: boolean;
  onLoadMore: () => void;
}

function LoadMore({ autoLoad = true, hasNext, loading, onLoadMore }: LoadMoreProps) {
  const sentinel = useRef<HTMLDivElement>(null);

  function requestMore() {
    if (hasNext && !loading) onLoadMore();
  }

  useEffect(() => {
    if (!autoLoad || !hasNext || loading || !sentinel.current) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting) requestMore();
    }, { rootMargin: "1600px 0px" });
    observer.observe(sentinel.current);
    return () => observer.disconnect();
  }, [autoLoad, hasNext, loading, onLoadMore]);

  if (!hasNext && !loading) return <div className="load-more-end">you’ve reached the end.</div>;
  return (
    <div className="load-more" ref={sentinel}>
      {loading ? <><span /> loading more…</> : <button className="button button-outlined" type="button" onClick={requestMore}>load more</button>}
    </div>
  );
}

interface ImageCardProps {
  post: Post;
  selectionMode: boolean;
  selected: boolean;
  downloading: boolean;
  onDownload: (post: Post) => Promise<void>;
  onSelect: (post: Post) => void;
  onToggleSelection: () => void;
  onTag: (tag: string) => void;
}

function ImageCard({ post, selectionMode, selected, downloading, onDownload, onSelect, onToggleSelection, onTag }: ImageCardProps) {
  const previewUrl = post.preview_url ?? post.sample_url ?? post.full_url;
  return (
    <article className={`card${selected ? " selected" : ""}`} onClick={() => selectionMode ? onToggleSelection() : onSelect(post)}>
      <div className="preview">
        {previewUrl ? (
          <img src={previewUrl} alt={`post ${post.post.id}`} loading="lazy" />
        ) : (
          <span className="missing-preview">preview unavailable</span>
        )}
        <span className="dimensions">{post.width ?? "?"}×{post.height ?? "?"}</span>
        <span className="rating-pill">{post.rating.toLowerCase()}</span>
        {selectionMode && (
          <button
            className="selection-toggle"
            type="button"
            aria-label={`${selected ? "deselect" : "select"} post ${post.post.id}`}
            aria-pressed={selected}
            onClick={(event) => {
              event.stopPropagation();
              onToggleSelection();
            }}
          >
            {selected && <Icon name="check" />}
          </button>
        )}
      </div>
      <div className="card-details">
        <div className="tag-list">
          {post.tags.slice(0, 4).map((tag) => (
            <button key={tag} className="tag-chip" type="button" onClick={(event) => {
              event.stopPropagation();
              onTag(tag);
            }}>{tag}</button>
          ))}
          {post.tags.length > 4 && <span className="tag-overflow">+{post.tags.length - 4}</span>}
        </div>
        <div className="card-footer">
          <span className="post-meta">#{post.post.id}{post.score !== null ? ` · ${post.score} score` : ""}</span>
          <button className="button button-primary" disabled={downloading} onClick={(event) => {
            event.stopPropagation();
            void onDownload(post);
          }}>
            {downloading ? "saving…" : "download"}
          </button>
        </div>
      </div>
    </article>
  );
}

interface PostInspectorProps {
  post: Post;
  detailLoading: boolean;
  detailError: string;
  downloading: boolean;
  siteName: string;
  favoriteSupported: boolean;
  favorited: boolean;
  onClose: () => void;
  canGoPrevious: boolean;
  canGoNext: boolean;
  previewPosition?: number;
  previewTotal: number;
  onPrevious: () => void;
  onNext: () => void;
  onOpenPost: (post: Post) => Promise<void>;
  onOpenSimilarSearch: () => Promise<void>;
  similarSearchSupported: boolean;
  onDownload: (post: Post) => Promise<void>;
  onFavorite: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
  relatedTagsSupported: boolean;
  relatedTagsOpen: boolean;
  relatedTags: RelatedTag[];
  relatedTagsLoading: boolean;
  relatedTagsError: string;
  onToggleRelatedTags: () => void;
  downloadVariant: MediaVariant;
}

function PostInspector({ post, detailLoading, detailError, downloading, siteName, favoriteSupported, favorited, onClose, canGoPrevious, canGoNext, previewPosition, previewTotal, onPrevious, onNext, onOpenPost, onOpenSimilarSearch, similarSearchSupported, onDownload, onFavorite, onTag, relatedTagsSupported, relatedTagsOpen, relatedTags, relatedTagsLoading, relatedTagsError, onToggleRelatedTags, downloadVariant }: PostInspectorProps) {
  const originalUrl = post.full_url ?? post.sample_url ?? post.preview_url;
  return (
    <aside className="detail-panel shell-surface" aria-label="post details">
      <div className="inspector-heading">
        <div>
          <p className="eyebrow">post details</p>
          <h2>#{post.post.id}</h2>
        </div>
        <button className="icon-button" type="button" aria-label="close details" onClick={onClose}><Icon name="close" /></button>
      </div>
      <div className="detail-preview">
        <DetailImage key={post.post.id} post={post} />
      </div>
      <div className="detail-navigation" aria-label="post preview navigation">
        <button className="icon-button detail-nav-button" type="button" aria-label="previous post" title="previous post" disabled={!canGoPrevious} onClick={onPrevious}><Icon name="back" /></button>
        <span>{previewPosition && previewTotal ? `${previewPosition} of ${previewTotal}` : "single post"}</span>
        <button className="icon-button detail-nav-button" type="button" aria-label="next post" title="next post" disabled={!canGoNext} onClick={onNext}><Icon name="forward" /></button>
      </div>
      <div className="detail-summary">
        <span>{siteName} post #{post.post.id}</span>
        {post.author && <button className="button button-text detail-author-link" type="button" title="search posts by this author" onClick={() => onTag(`user:${post.author}`)}>author: {post.author}</button>}
        <button className="button button-outlined detail-post-link" type="button" onClick={() => void onOpenPost(post)}>open {siteName} post</button>
        {similarSearchSupported && <button className="button button-outlined detail-post-link" type="button" onClick={() => void onOpenSimilarSearch()}>open similar search</button>}
        {originalUrl && <a href={originalUrl} target="_blank" rel="noreferrer">open original</a>}
        {post.source && <a href={post.source} target="_blank" rel="noreferrer">open source</a>}
      </div>
      {(post.parent_id || post.has_children) && (
        <div className="detail-related-actions" aria-label="related posts">
          {post.parent_id && <button className="button button-outlined detail-post-link" type="button" onClick={() => onTag(`id:${post.parent_id}`)}>find parent #{post.parent_id}</button>}
          {post.has_children && <button className="button button-outlined detail-post-link" type="button" onClick={() => onTag(`parent:${post.post.id}`)}>find child posts</button>}
        </div>
      )}
      {relatedTagsSupported && (
        <div className="detail-related-tags">
          <button className="button button-outlined detail-post-link" type="button" onClick={onToggleRelatedTags}>
            {relatedTagsOpen ? "hide related tags" : "show related tags"}
          </button>
          {relatedTagsOpen && <p className="detail-helper">site metadata may include tags outside the current rating filter; post results still follow content policy.</p>}
          {relatedTagsOpen && relatedTagsLoading && <p className="detail-helper" role="status">loading related tags…</p>}
          {relatedTagsOpen && relatedTagsError && <p className="detail-helper" role="alert">couldn’t load related tags. {relatedTagsError}</p>}
          {relatedTagsOpen && !relatedTagsLoading && !relatedTagsError && relatedTags.length > 0 && (
            <div className="tag-list" aria-label="related tags">
              {relatedTags.map((tag) => (
                <button key={tag.name} className="tag-chip" type="button" title={tag.post_count === null ? undefined : `${tag.post_count.toLocaleString()} posts`} onClick={() => onTag(tag.name)}>{tag.name}</button>
              ))}
            </div>
          )}
          {relatedTagsOpen && !relatedTagsLoading && !relatedTagsError && relatedTags.length === 0 && <p className="detail-helper">no related tags found.</p>}
        </div>
      )}
      {detailLoading && <p className="detail-helper" role="status">refreshing post details…</p>}
      {detailError && <p className="detail-helper" role="alert">couldn’t refresh the post; showing the feed snapshot. {detailError}</p>}
      <div className="inspector-section">
        <span className="section-label">tags</span>
        <div className="tag-list">
          {post.tags.map((tag) => <button key={tag} className="tag-chip" type="button" onClick={() => onTag(tag)}>{tag}</button>)}
        </div>
      </div>
      <dl className="metadata">
        <div><dt>site</dt><dd>{siteName}</dd></div>
        <div><dt>post id</dt><dd>{post.post.id}</dd></div>
        <div><dt>author</dt><dd>{post.author ?? "—"}{post.creator_id ? ` · #${post.creator_id}` : ""}</dd></div>
        <div><dt>rating</dt><dd>{post.rating.toLowerCase()}</dd></div>
        <div><dt>size</dt><dd>{post.width ?? "?"}×{post.height ?? "?"}</dd></div>
        <div><dt>score</dt><dd>{post.score ?? "—"}</dd></div>
        <div><dt>file size</dt><dd>{post.file_size ? `${Math.round(post.file_size / 1024)} kb` : "—"}</dd></div>
        <div><dt>md5</dt><dd className="metadata-value">{post.md5 ?? "—"}</dd></div>
        <div><dt>parent</dt><dd>{post.parent_id ? `#${post.parent_id}` : "none"}</dd></div>
        <div><dt>children</dt><dd>{post.has_children ? "yes" : "no"}</dd></div>
        <div><dt>created</dt><dd>{post.created_at ? new Date(post.created_at).toLocaleString() : "—"}</dd></div>
      </dl>
      {!detailLoading && !detailError && <p className="detail-helper">post details are hydrated from the site API.</p>}
      {favoriteSupported && (
        <button className="button button-outlined button-wide" type="button" onClick={() => void onFavorite(post)}>
          {favorited ? `remove from ${siteName} favorites` : `add to ${siteName} favorites`}
        </button>
      )}
      <button className="button button-primary button-wide" disabled={downloading} onClick={() => void onDownload(post)}>
        {downloading ? "saving…" : `download ${downloadVariant.toLowerCase()} quality`}
      </button>
    </aside>
  );
}

function DetailImage({ post }: { post: Post }) {
  const sources = [post.sample_url, post.preview_url, post.full_url].filter(
    (url, index, all): url is string => Boolean(url) && all.indexOf(url) === index,
  );
  const [sourceIndex, setSourceIndex] = useState(0);
  const source = sources[sourceIndex];

  if (!source) return <span>preview unavailable</span>;
  return (
    <img
      src={source}
      alt={`post ${post.post.id}`}
      loading="eager"
      onError={() => setSourceIndex((current) => current + 1)}
    />
  );
}

interface DownloadPanelProps {
  records: DownloadRecord[];
  archives: ArchiveRecord[];
  historyHasNext: boolean;
  historyLoading: boolean;
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
  onOpen: (path: string) => Promise<void>;
  onLoadMore: () => void;
  onCancelArchive: (id: string) => Promise<void>;
}

function DownloadPanel({ records, archives, historyHasNext, historyLoading, onCancel, onRetry, onOpen, onLoadMore, onCancelArchive }: DownloadPanelProps) {
  const active = records.filter((record) => isActiveDownload(record.status));
  const history = records.filter((record) => !isActiveDownload(record.status));
  const activeArchives = archives.filter((record) => isActiveDownload(record.status));
  const archiveHistory = archives.filter((record) => !isActiveDownload(record.status));
  const groups = new Map<string, DownloadRecord[]>();
  for (const record of history) {
    const date = new Date(record.created_at_ms);
    const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
    groups.set(key, [...(groups.get(key) ?? []), record]);
  }
  const historyGroups = [...groups.entries()].sort(([a], [b]) => b.localeCompare(a));
  return (
    <section className="workspace-panel shell-surface" aria-label="downloads">
      <div className="inspector-heading">
        <div><p className="eyebrow">local state</p><h2>downloads</h2></div>
      </div>
      <p className="helper-text">{active.length + activeArchives.length ? `${active.length + activeArchives.length} item${active.length + activeArchives.length === 1 ? "" : "s"} in progress` : "nothing is downloading"}</p>
      {records.length === 0 && archives.length === 0 ? (
        <div className="panel-empty">your download history will appear here.</div>
      ) : (
        <>
          {active.length + activeArchives.length > 0 && <section className="download-section">
            <h3 className="download-section-title">in progress</h3>
            <div className="download-list">
              {activeArchives.map((record) => <ArchiveRow key={record.id} record={record} onCancel={onCancelArchive} onOpen={onOpen} />)}
              {active.map((record) => <DownloadRow key={record.id} record={record} onCancel={onCancel} onRetry={onRetry} onOpen={onOpen} />)}
            </div>
          </section>}
          {archiveHistory.length > 0 && <section className="download-section">
            <h3 className="download-section-title">pool archives</h3>
            <div className="download-list">
              {archiveHistory.map((record) => <ArchiveRow key={record.id} record={record} onCancel={onCancelArchive} onOpen={onOpen} />)}
            </div>
          </section>}
          {historyGroups.length > 0 && <section className="download-section">
            <h3 className="download-section-title">history</h3>
            <div className="download-history">
              {historyGroups.map(([key, group]) => {
                const [year, month] = key.split("-");
                const monthName = new Date(Number(year), Number(month) - 1, 1).toLocaleString(undefined, { month: "long" });
                return <section className="download-month" key={key}>
                  <h4>{year}<span>{monthName}</span></h4>
                  <div className="download-list">
                    {group.map((record) => <DownloadRow key={record.id} record={record} onCancel={onCancel} onRetry={onRetry} onOpen={onOpen} />)}
                  </div>
                </section>;
              })}
            </div>
            <LoadMore hasNext={historyHasNext} loading={historyLoading} onLoadMore={onLoadMore} />
          </section>}
        </>
      )}
    </section>
  );
}

interface DownloadRowProps {
  record: DownloadRecord;
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
  onOpen: (path: string) => Promise<void>;
}

function DownloadRow({ record, onCancel, onRetry, onOpen }: DownloadRowProps) {
  const canCancel = record.status === "Queued" || record.status === "Running";
  const canRetry = record.status === "Failed" || record.status === "Cancelled";
  const canOpen = Boolean(record.target_path) && (record.status === "Completed" || record.status === "ExistingTarget");
  return (
    <article className="download-row">
      <div className="download-row-heading">
        <strong>#{record.post_id}</strong>
        <span className={`download-status status-${record.status.toLowerCase()}`}>{downloadStatusLabel(record.status)}</span>
      </div>
      <p>{record.variant} quality · attempt {record.attempts || 1}</p>
      {record.error && <p className="download-error">{record.error}</p>}
      {record.target_path && <p className="download-path" title={record.target_path}>{record.target_path}</p>}
      {(canCancel || canRetry || canOpen) && <div className="download-row-actions">
        {canCancel && <button className="button button-text" type="button" onClick={() => void onCancel(record.id)}>cancel</button>}
        {canRetry && <button className="button button-outlined" type="button" onClick={() => void onRetry(record.id)}>retry</button>}
        {canOpen && <button className="button button-outlined" type="button" onClick={() => void onOpen(record.target_path!)}>open file</button>}
      </div>}
    </article>
  );
}

interface ArchiveRowProps {
  record: ArchiveRecord;
  onCancel: (id: string) => Promise<void>;
  onOpen: (path: string) => Promise<void>;
}

function ArchiveRow({ record, onCancel, onOpen }: ArchiveRowProps) {
  const canCancel = record.status === "Queued" || record.status === "Running";
  const canOpen = Boolean(record.target_path) && (record.status === "Completed" || record.status === "ExistingTarget");
  return (
    <article className="download-row archive-row">
      <div className="download-row-heading">
        <strong>{record.pool_name}</strong>
        <span className={`download-status status-${record.status.toLowerCase()}`}>{downloadStatusLabel(record.status)}</span>
      </div>
      <p>pool zip · {record.pool_id} · attempt {record.attempts || 1}</p>
      {record.error && <p className="download-error">{record.error}</p>}
      {record.target_path && <p className="download-path" title={record.target_path}>{record.target_path}</p>}
      {(canCancel || canOpen) && <div className="download-row-actions">
        {canCancel && <button className="button button-text" type="button" onClick={() => void onCancel(record.id)}>cancel</button>}
        {canOpen && <button className="button button-outlined" type="button" onClick={() => void onOpen(record.target_path!)}>open file</button>}
      </div>}
    </article>
  );
}

interface PoolPanelProps {
  pools: Pool[];
  poolsLoading: boolean;
  poolsError: string;
  poolsHasNext: boolean;
  selectedPool: Pool | null;
  posts: Post[];
  postsLoading: boolean;
  postsError: string;
  postsHasNext: boolean;
  onRetryPools: () => void;
  onRetryPosts: () => void;
  onLoadMorePools: () => void;
  onLoadMorePosts: () => void;
  onBack: () => void;
  onBrowse: (pool: Pool) => void;
  onDownloadZip: (pool: Pool) => void;
  onSelectPost: (post: Post) => void;
  onDownload: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
  downloadingId: string | null;
  siteName: string;
  collectionDownloads: boolean;
  poolSearch: string;
  onPoolSearchChange: (value: string) => void;
  onSubmitPoolSearch: () => void;
  onClearPoolSearch: () => void;
}

function PoolPanel({ pools, poolsLoading, poolsError, poolsHasNext, selectedPool, posts, postsLoading, postsError, postsHasNext, onRetryPools, onRetryPosts, onLoadMorePools, onLoadMorePosts, onBack, onBrowse, onDownloadZip, onSelectPost, onDownload, onTag, downloadingId, siteName, collectionDownloads, poolSearch, onPoolSearchChange, onSubmitPoolSearch, onClearPoolSearch }: PoolPanelProps) {
  if (selectedPool) {
    return (
      <section className="workspace-panel shell-surface" aria-label={`${selectedPool.name} pool`}>
        <div className="inspector-heading">
          <div><p className="eyebrow">{siteName} pool</p><h2>{selectedPool.name}</h2></div>
          <div className="inspector-actions">
            {collectionDownloads && <button className="button button-outlined" type="button" onClick={() => onDownloadZip(selectedPool)}>download zip</button>}
            <button className="button button-outlined button-with-icon" type="button" onClick={onBack}><Icon name="back" /><span>all pools</span></button>
          </div>
        </div>
        <p className="helper-text">{selectedPool.post_count} ordered post{selectedPool.post_count === 1 ? "" : "s"} from {siteName}.</p>
        {postsError && <div className="panel-error" role="alert"><p>{postsError}</p><button className="button button-outlined" type="button" onClick={onRetryPosts}>try again</button></div>}
        {postsLoading && posts.length === 0 && <p className="loading-line" role="status"><span /> loading pool posts…</p>}
        {!postsLoading && !postsError && posts.length === 0 && <div className="panel-empty">this pool has no visible posts.</div>}
        <div className="gallery-grid">
          {posts.map((post) => (
            <ImageCard
              key={post.post.id}
              post={post}
              selectionMode={false}
              selected={false}
              downloading={downloadingId === post.post.id}
              onDownload={onDownload}
              onSelect={onSelectPost}
              onToggleSelection={() => undefined}
              onTag={onTag}
            />
          ))}
        </div>
        {posts.length > 0 && <LoadMore autoLoad={false} hasNext={postsHasNext} loading={postsLoading} onLoadMore={onLoadMorePosts} />}
      </section>
    );
  }

  return (
    <section className="workspace-panel shell-surface" aria-label="pools">
      <div className="inspector-heading">
        <div><p className="eyebrow">{siteName} collections</p><h2>pools</h2></div>
      </div>
      <form className="pool-search" role="search" onSubmit={(event) => { event.preventDefault(); onSubmitPoolSearch(); }}>
        <input aria-label="search pools" placeholder="search pools…" value={poolSearch} onChange={(event) => onPoolSearchChange(event.target.value)} />
        <button className="button button-primary" type="submit">search</button>
        {poolSearch && <button className="button button-text" type="button" onClick={onClearPoolSearch}>clear</button>}
      </form>
      <p className="helper-text">public pools group ordered posts from {siteName}. open a pool to browse its ordered posts{collectionDownloads ? " or request its authenticated zip archive" : ""}.</p>
      {poolsLoading && pools.length === 0 && <p className="loading-line" role="status"><span /> loading pools…</p>}
      {poolsError && <div className="panel-error" role="alert"><p>{poolsError}</p><button className="button button-outlined" type="button" onClick={onRetryPools}>try again</button></div>}
      {!poolsLoading && !poolsError && pools.length === 0 && <div className="panel-empty">no public pools found.</div>}
      <div className="collection-list">
        {pools.map((pool) => (
          <article className="collection-row" key={pool.id}>
            <div><strong>{pool.name}</strong><p>{pool.post_count} post{pool.post_count === 1 ? "" : "s"}</p></div>
            <button className="button button-outlined" type="button" onClick={() => onBrowse(pool)}>browse</button>
          </article>
        ))}
      </div>
      {pools.length > 0 && <LoadMore autoLoad={false} hasNext={poolsHasNext} loading={poolsLoading} onLoadMore={onLoadMorePools} />}
    </section>
  );
}

interface AccountPanelProps {
  auth: AuthStatus | null;
  favorites: Post[];
  favoritesLoading: boolean;
  favoritesError: string;
  favoritesHasNext: boolean;
  loading: boolean;
  onBeginAuth: () => void;
  onRefresh: () => void;
  onRetry: () => void;
  onLoadMore: () => void;
  onSelect: (post: Post) => void;
  onDownload: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
  onSignOut: () => Promise<void>;
}

function AccountPanel({ auth, favorites, favoritesLoading, favoritesError, favoritesHasNext, loading, onBeginAuth, onRefresh, onRetry, onLoadMore, onSelect, onDownload, onTag, onSignOut }: AccountPanelProps) {
  return (
    <section className="workspace-panel shell-surface" aria-label="favorites account">
      <div className="inspector-heading">
        <div><p className="eyebrow">yandere account</p><h2>favorites</h2></div>
      </div>
      {auth?.authenticated ? (
        <>
          <p className="account-connected"><span className="connection-dot" /> {auth.username ? `${auth.username} connected` : "yandere account connected"}</p>
          {favoritesError && <div className="panel-error" role="alert"><p>{favoritesError}</p><button className="button button-outlined" type="button" onClick={onRetry}>try again</button></div>}
          {favoritesLoading && favorites.length === 0 && <p className="helper-text">loading favorites…</p>}
          {!favoritesLoading && favorites.length === 0 && <div className="panel-empty">no favorites found.</div>}
          <div className="gallery-grid">
            {favorites.map((post) => (
              <ImageCard
                key={post.post.id}
                post={post}
                selectionMode={false}
                selected={false}
                downloading={false}
                onDownload={onDownload}
                onSelect={onSelect}
                onToggleSelection={() => undefined}
                onTag={onTag}
              />
            ))}
          </div>
          <LoadMore hasNext={favoritesHasNext} loading={favoritesLoading && favorites.length > 0} onLoadMore={onLoadMore} />
          <button className="button button-outlined" type="button" onClick={() => void onSignOut()}>sign out</button>
        </>
      ) : (
        <>
          <p className="helper-text">sign in through yandere’s own page. Dreamland reads only the safe auth state and keeps the browser session in Rust.</p>
          <button className="button button-primary button-wide" type="button" onClick={onBeginAuth}>sign in to yandere</button>
          <button className="button button-outlined button-wide" type="button" disabled={loading} onClick={onRefresh}>{loading ? "checking…" : "check login"}</button>
        </>
      )}
    </section>
  );
}

function downloadStatusLabel(status: DownloadStatus): string {
  switch (status) {
    case "ExistingTarget": return "already exists";
    case "Queued": return "queued";
    case "Running": return "downloading";
    case "Completed": return "completed";
    case "Failed": return "failed";
    case "Cancelled": return "cancelled";
  }
}

interface SettingsDialogProps {
  config: AppConfig | null;
  onCancel: () => void;
  onSave: (downloadPath: string, contentPolicy: ContentPolicy, downloadVariant: MediaVariant, network: NetworkPolicy) => Promise<void>;
}

function SettingsDialog({ config, onCancel, onSave }: SettingsDialogProps) {
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
  const [contentPolicy, setContentPolicy] = useState<ContentPolicy>(config?.content_policy ?? "SafeOnly");
  const [downloadVariant, setDownloadVariant] = useState<MediaVariant>(config?.download_variant ?? "Full");
  const configuredProxy = config?.network.proxy ?? "Auto";
  const [proxyMode, setProxyMode] = useState<"Auto" | "Direct" | "Manual">(
    typeof configuredProxy === "string" ? configuredProxy : "Manual",
  );
  const [proxyUrl, setProxyUrl] = useState(typeof configuredProxy === "string" ? "" : configuredProxy.Manual.url);
  const [maxRetries, setMaxRetries] = useState(config?.network.max_retries ?? 2);
  const [retryDelayMs, setRetryDelayMs] = useState(config?.network.retry_delay_ms ?? 500);
  const [maxRetryDelayMs, setMaxRetryDelayMs] = useState(config?.network.max_retry_delay_ms ?? 8000);
  const [proxyDetection, setProxyDetection] = useState<string | null>(null);
  const [detecting, setDetecting] = useState(false);
  const [saving, setSaving] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    const proxy: ProxyMode = proxyMode === "Manual" ? { Manual: { url: proxyUrl } } : proxyMode;
    await onSave(downloadPath, contentPolicy, downloadVariant, { proxy, max_retries: maxRetries, retry_delay_ms: retryDelayMs, max_retry_delay_ms: maxRetryDelayMs });
    setSaving(false);
  }

  async function handleDetectProxy() {
    setDetecting(true);
    try {
      const result = await detectProxy();
      setProxyDetection(result.detected ? `detected ${result.endpoint ?? "a system proxy"} (${result.source ?? "system"})` : "no proxy detected");
    } catch (reason) {
      setProxyDetection(`detection failed: ${errorMessage(reason)}`);
    } finally {
      setDetecting(false);
    }
  }

  return (
    <div className="settings-overlay" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onCancel();
    }}>
      <form className="settings-dialog shell-surface" onSubmit={submit} role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <div className="inspector-heading">
          <div><p className="eyebrow">app preferences</p><h2 id="settings-title">settings</h2></div>
          <button className="icon-button" type="button" aria-label="close settings" onClick={onCancel}><Icon name="close" /></button>
        </div>
        <div className="settings-section">
          <p className="section-label">storage & site</p>
          <label className="field">download path<input required value={downloadPath} onChange={(event) => setDownloadPath(event.target.value)} /></label>
          <label className="field">content policy
            <select value={contentPolicy} onChange={(event) => setContentPolicy(event.target.value as ContentPolicy)}>
              <option value="SafeOnly">safe only (default)</option>
              <option value="AllowQuestionable">allow questionable</option>
              <option value="AllowExplicit">allow explicit</option>
              <option value="ExplicitOnly">explicit only</option>
            </select>
          </label>
          <label className="field">download quality
            <select value={downloadVariant} onChange={(event) => setDownloadVariant(event.target.value as MediaVariant)}>
              <option value="Full">best available (full)</option>
              <option value="Sample">sample</option>
              <option value="Preview">preview</option>
            </select>
          </label>
          <p className="helper-text">this controls which ratings appear in feeds and searches.</p>
        </div>
        <div className="settings-section">
          <p className="section-label">connection resilience</p>
          <label className="field">proxy
            <select value={proxyMode} onChange={(event) => setProxyMode(event.target.value as "Auto" | "Direct" | "Manual")}>
              <option value="Auto">use system / environment</option>
              <option value="Direct">direct connection</option>
              <option value="Manual">manual proxy</option>
            </select>
          </label>
          {proxyMode === "Manual" && <label className="field">proxy url<input required placeholder="http://127.0.0.1:7890" value={proxyUrl} onChange={(event) => setProxyUrl(event.target.value)} /></label>}
          <button className="button button-outlined" type="button" disabled={detecting} onClick={() => void handleDetectProxy()}>{detecting ? "detecting…" : "detect proxy"}</button>
          {proxyDetection && <p className="helper-text" role="status">{proxyDetection}</p>}
          <div className="field-grid">
            <label className="field">max retries<input type="number" min="0" max="8" value={maxRetries} onChange={(event) => setMaxRetries(Number(event.target.value))} /></label>
            <label className="field">initial delay<input type="number" min="100" value={retryDelayMs} onChange={(event) => setRetryDelayMs(Number(event.target.value))} /></label>
          </div>
          <label className="field">maximum retry delay<input type="number" min="100" value={maxRetryDelayMs} onChange={(event) => setMaxRetryDelayMs(Number(event.target.value))} /></label>
          <p className="helper-text">429 and temporary server responses retry with a bounded delay.</p>
        </div>
        <div className="dialog-actions">
          <button className="button button-text" type="button" onClick={onCancel}>cancel</button>
          <button className="button button-primary" type="submit" disabled={saving}>{saving ? "saving…" : "save settings"}</button>
        </div>
      </form>
    </div>
  );
}

export default App;

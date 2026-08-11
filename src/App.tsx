import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  authStatus,
  beginAuth,
  cancelQuery,
  continueQuery,
  detectProxy,
  enqueueDownload,
  listPools,
  listFavorites,
  listDownloads,
  listDownloadHistory,
  listSavedQueries,
  cancelDownload,
  deleteSavedQuery,
  openDownload,
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
  type SavedQuery,
} from "./lib/ipc";

type ViewMode = "latest" | "popular" | "search" | "downloads" | "pools" | "favorites";
type PopularPeriod = "Day" | "Week" | "Month";
type ToastTone = "success" | "info" | "error";
type IconName = "clock" | "trend" | "download" | "book" | "heart" | "search" | "refresh" | "settings" | "close" | "back" | "check";

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
  if (source === "Browse") return "Latest posts";
  if ("Search" in source) return source.Search.expression;
  if (source.Feed.kind === "Latest") return "Latest feed";
  return `${source.Feed.kind.Popular.period} popular feed`;
}

function savedQueryIsRunnable(saved: SavedQuery): boolean {
  return savedQueryExpression(saved) !== null;
}

function contentPolicyLabel(policy: ContentPolicy): string {
  switch (policy) {
    case "SafeOnly": return "Safe only";
    case "AllowQuestionable": return "Safe + questionable";
    case "AllowExplicit": return "All ratings";
    case "ExplicitOnly": return "Explicit only";
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
    check: "m5 12 4 4L19 6",
  };
  const material = name === "refresh" || name === "settings";
  return <svg className="icon" viewBox="0 0 24 24" fill={material ? "currentColor" : "none"} stroke={material ? "none" : "currentColor"} strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}

function App() {
  const queryClient = useQueryClient();
  const [view, setView] = useState<ViewMode>("latest");
  const [popularPeriod, setPopularPeriod] = useState<PopularPeriod>("Week");
  const [popularAnchorDate, setPopularAnchorDate] = useState(() => normalizePopularAnchor(today(), "Week"));
  const [selectedSavedQueryId, setSelectedSavedQueryId] = useState<string | null>(null);
  const [searchDraft, setSearchDraft] = useState("");
  const [submittedSearch, setSubmittedSearch] = useState("");
  const [searchFocused, setSearchFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
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
  const authQuery = useQuery({
    queryKey: ["auth"],
    queryFn: authStatus,
  });
  const savedQueriesQuery = useQuery({
    queryKey: ["saved-queries"],
    queryFn: listSavedQueries,
    enabled: configQuery.isSuccess,
  });
  const pageSize = configQuery.data?.images_per_page ?? 20;
  const contentPolicy = configQuery.data?.content_policy ?? "SafeOnly";
  const activeSavedQuery = savedQueriesQuery.data?.find((saved) => saved.id === selectedSavedQueryId);
  const activeContentPolicy = activeSavedQuery?.query.content_policy ?? contentPolicy;
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
    queryKey: ["posts", request],
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) => pageParam ? continueQuery(pageParam) : queryPosts(request),
    getNextPageParam: (lastPage) => lastPage.session && lastPage.posts.length >= lastPage.page_size
      ? lastPage.session
      : undefined,
    enabled: configQuery.isSuccess && (view === "latest" || view === "popular" || view === "search"),
  });
  useEffect(() => {
    activeQuerySession.current = imagesQuery.data?.pages.at(-1)?.session ?? null;
  }, [imagesQuery.data]);
  useEffect(() => {
    return () => {
      const session = activeQuerySession.current;
      if (session) void cancelQuery(session).catch(() => undefined);
    };
  }, [request]);
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
  useEffect(() => {
    const records = downloadsQuery.data?.pages.flatMap((page) => page) ?? [];
    if (!records.length) return;
    for (const record of records) {
      const previous = downloadStatuses.current.get(record.id);
      if (previous && previous !== record.status) {
        if (record.status === "Completed") {
          showToast("Download complete", `Post #${record.post_id} is ready in Downloads.`);
        } else if (record.status === "ExistingTarget") {
          showToast("Already downloaded", `Post #${record.post_id} was not overwritten.`, "info");
        } else if (record.status === "Failed") {
          showToast("Download failed", record.error ?? `Post #${record.post_id} could not be saved.`, "error");
        }
      }
      downloadStatuses.current.set(record.id, record.status);
    }
  }, [downloadsQuery.data]);
  const suggestionsQuery = useQuery({
    queryKey: ["tag-suggestions", searchDraft.trim()],
    queryFn: () => suggestTags(searchDraft.trim(), 7),
    enabled: searchFocused && searchDraft.trim().length >= 1,
    staleTime: 30_000,
  });
  const poolsQuery = useInfiniteQuery({
    queryKey: ["pools"],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => listPools(pageParam, 30),
    getNextPageParam: (lastPage, pages) => lastPage.has_next ? pages.length + 1 : undefined,
    enabled: view === "pools" && selectedPool === null,
  });
  const poolPostsQuery = useInfiniteQuery({
    queryKey: ["pool-posts", selectedPool?.id, contentPolicy],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => queryPoolPosts(selectedPool!.id, pageParam, pageSize),
    getNextPageParam: (lastPage, pages) => lastPage.posts.length >= lastPage.page_size ? pages.length + 1 : undefined,
    enabled: view === "pools" && selectedPool !== null && configQuery.isSuccess,
  });
  const favoritesQuery = useInfiniteQuery({
    queryKey: ["favorites"],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => listFavorites(pageParam, 20),
    getNextPageParam: (lastPage, pages) => lastPage.posts.length >= lastPage.page_size ? pages.length + 1 : undefined,
    enabled: view === "favorites" && authQuery.data?.authenticated === true && Boolean(authQuery.data.username),
  });
  useEffect(() => {
    const posts = favoritesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
    if (!posts.length) return;
    setFavoritePostIds((current) => new Set([...current, ...posts.map((post) => post.post.id)]));
  }, [favoritesQuery.data]);
  const saveConfigMutation = useMutation({
    mutationFn: ({ downloadPath, contentPolicy, network }: ConfigInput) =>
      saveConfig(downloadPath, contentPolicy, network),
    onSuccess: (nextConfig) => {
      queryClient.setQueryData(["config"], nextConfig);
      setSettingsOpen(false);
      setNotice("Settings saved");
      showToast("Settings saved", "Your preferences are now active.");
    },
    onError: (reason) => setError(`Failed to save settings: ${errorMessage(reason)}`),
  });
  const downloadMutation = useMutation({
    mutationFn: ({ postId, variant }: DownloadInput) => enqueueDownload(postId, variant),
    onSuccess: (record) => {
      void queryClient.invalidateQueries({ queryKey: ["downloads"] });
      downloadStatuses.current.set(record.id, record.status);
      setNotice(`Added post #${record.post_id} to the download queue`);
      showToast("Download queued", `Post #${record.post_id} will be saved at the configured path.`);
    },
    onError: (reason) => setError(`Download failed: ${errorMessage(reason)}`),
  });

  const isBrowseView = view === "latest" || view === "popular" || view === "search";
  const queryError = configQuery.error
    ? `Failed to load configuration: ${errorMessage(configQuery.error)}`
    : isBrowseView && imagesQuery.error
      ? `Failed to load images: ${errorMessage(imagesQuery.error)}`
      : "";
  const images = imagesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const favoritePosts = favoritesQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const pools = poolsQuery.data?.pages.flatMap((page) => page.pools).filter((pool) => pool.public) ?? [];
  const poolPosts = poolPostsQuery.data?.pages.flatMap((page) => page.posts) ?? [];
  const selectedPosts = images.filter((post) => selectedPostIds.has(post.post.id));
  const loading = configQuery.isPending || imagesQuery.isPending;
  const loadingMore = imagesQuery.isFetchingNextPage;
  const downloadRecords = useMemo(() => {
    const records = downloadsQuery.data?.pages.flatMap((page) => page) ?? [];
    return [...new Map(records.map((record) => [record.id, record])).values()];
  }, [downloadsQuery.data]);
  const title = view === "search"
    ? "Search results"
    : view === "popular"
      ? "Popular"
      : view === "downloads"
        ? "Downloads"
        : view === "pools"
          ? "Pools"
          : view === "favorites"
            ? "Favorites"
            : "Latest posts";
  const subtitle = view === "search"
    ? `Matching “${submittedSearch}”`
    : view === "popular"
      ? `Most popular this ${popularPeriod.toLowerCase()} · ${popularWindow(popularAnchorDate, popularPeriod).join(" to ")} · score-ranked`
      : view === "downloads"
        ? "Local download history and active work"
        : view === "pools"
          ? "Ordered public collections from Yande"
          : view === "favorites"
            ? authQuery.data?.authenticated ? `Saved by ${authQuery.data.username ?? "your Yande account"}` : "Sign in to browse your saved posts"
            : "A calm feed for finding something worth keeping";

  function changeView(nextView: ViewMode) {
    setError("");
    setNotice("");
    setView(nextView);
    setSelectedSavedQueryId(null);
    setSelectedPool(null);
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
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function chooseTag(tag: string) {
    setSearchDraft(tag);
    setSubmittedSearch(tag);
    setSelectedSavedQueryId(null);
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function openSavedQuery(saved: SavedQuery) {
    const expression = savedQueryExpression(saved);
    if (!expression) return;
    setSelectedSavedQueryId(saved.id);
    setSearchDraft(expression);
    setSubmittedSearch(expression);
    setView("search");
    setError("");
    setNotice("");
  }

  async function handleSaveQuery() {
    if (view !== "search" || !submittedSearch) return;
    const name = window.prompt("Name this saved query", activeSavedQuery?.name ?? submittedSearch.trim());
    if (!name?.trim()) return;
    try {
      const saved = await saveSavedQuery({
        id: activeSavedQuery?.id ?? "",
        site: "yandere",
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
      setNotice(`Saved query “${saved.name}”`);
    } catch (reason) {
      setError(`Could not save query: ${errorMessage(reason)}`);
    }
  }

  async function handleToggleSavedPin(saved: SavedQuery) {
    try {
      await saveSavedQuery({ ...saved, pinned: !saved.pinned });
      await savedQueriesQuery.refetch();
    } catch (reason) {
      setError(`Could not update saved query: ${errorMessage(reason)}`);
    }
  }

  async function handleDeleteSavedQuery(saved: SavedQuery) {
    if (!window.confirm(`Delete “${saved.name}”?`)) return;
    try {
      await deleteSavedQuery(saved.id);
      await savedQueriesQuery.refetch();
      if (selectedSavedQueryId === saved.id) {
        setSelectedSavedQueryId(null);
        setView("latest");
      }
    } catch (reason) {
      setError(`Could not delete query: ${errorMessage(reason)}`);
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
        await downloadMutation.mutateAsync({ postId: post.post.id, variant: "Full" });
        queued += 1;
      } catch {
        // Each failed item is reported by the mutation; continue the batch.
      }
    }
    setBatchDownloading(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
    if (queued > 0) showToast("Batch queued", `${queued} post${queued === 1 ? "" : "s"} added to Downloads.`);
  }

  async function handleFavorite(post: Post) {
    if (!authQuery.data?.authenticated) {
      setView("favorites");
      showToast("Sign in required", "Connect your Yande account before changing favorites.", "info");
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
      showToast(favorite ? "Added to favorites" : "Removed from favorites", `Post #${post.post.id} updated on Yande.`);
    } catch (reason) {
      setError(`Favorite failed: ${errorMessage(reason)}`);
    }
  }

  async function handleDownload(post: Post) {
    setDownloadingId(post.post.id);
    setError("");
    setNotice("");
    try {
      await downloadMutation.mutateAsync({ postId: post.post.id, variant: "Full" });
    } catch {
      // The mutation reports the user-facing error.
    } finally {
      setDownloadingId(null);
    }
  }

  async function handleSaveConfig(
    downloadPath: string,
    contentPolicy: ContentPolicy,
    network: NetworkPolicy,
  ) {
    setError("");
    setNotice("");
    try {
      await saveConfigMutation.mutateAsync({ downloadPath, contentPolicy, network });
    } catch {
      // The mutation reports the user-facing error.
    }
  }

  return (
    <div className="app">
      <header className="app-header shell-surface">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">✦</div>
          <div>
            <p className="eyebrow">Yandere / image board</p>
            <h1>Dreamland</h1>
          </div>
        </div>
        <form className="search-bar" onSubmit={submitSearch} role="search">
          <span className="search-icon"><Icon name="search" /></span>
          <input
            ref={searchInput}
            aria-label="Search tags"
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
              aria-label="Clear search"
              onClick={() => {
                setSearchDraft("");
                changeView("latest");
              }}
            >
              ×
            </button>
          )}
          {searchFocused && suggestionsQuery.data && suggestionsQuery.data.length > 0 && (
            <div className="suggestions" role="listbox">
              <p className="suggestion-heading">Suggested tags</p>
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
            aria-label={`Refresh ${title.toLowerCase()}`}
            title={`Refresh ${title.toLowerCase()}`}
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
            <span>Settings</span>
          </button>
        </div>
      </header>

      <div className={`app-layout${selectedPost ? " has-detail" : ""}`}>
        <main className="content">
          <nav className="view-tabs shell-surface" aria-label="Dreamland sections" role="tablist">
          <NavButton active={view === "latest"} label="Latest" icon="clock" onClick={() => changeView("latest")} />
          <NavButton active={view === "popular"} label="Popular" icon="trend" onClick={() => changeView("popular")} />
          {savedQueriesQuery.data?.map((saved) => (
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
                {!savedQueryIsRunnable(saved) && <small>Unavailable</small>}
                {saved.pinned && savedQueryIsRunnable(saved) && <small>Pinned</small>}
              </button>
              <div className="saved-query-actions">
                <button className="icon-button" type="button" aria-label={`${saved.pinned ? "Unpin" : "Pin"} ${saved.name}`} onClick={() => void handleToggleSavedPin(saved)}>
                  {saved.pinned ? "•" : "○"}
                </button>
                <button className="icon-button" type="button" aria-label={`Delete ${saved.name}`} onClick={() => void handleDeleteSavedQuery(saved)}>×</button>
              </div>
            </div>
          ))}
          <NavButton active={view === "pools"} label="Pools" icon="book" onClick={() => changeView("pools")} />
          <NavButton active={view === "favorites"} label="Favorites" icon="heart" onClick={() => changeView("favorites")} />
          <NavButton active={view === "downloads"} label="Downloads" icon="download" onClick={() => changeView("downloads")} />
          <span className="view-status">
            <span className="connection-dot" aria-hidden="true" />
            <span>Yandere connected</span>
          </span>
          </nav>
          <section className="content-heading">
            <div>
              <p className="eyebrow">Explore freely</p>
              <h2>{title}</h2>
              <p className="subtitle">{subtitle}</p>
            </div>
            <div className="heading-actions">
              {isBrowseView && <>
              <button className="button button-outlined" type="button" onClick={() => {
                setSelectionMode((current) => !current);
                setSelectedPostIds(new Set());
              }}>
                {selectionMode ? "Cancel selection" : "Select posts"}
              </button>
              {view === "popular" && (
                <div className="popular-controls" aria-label="Popular period and date">
                  <label className="period-picker">
                    <span>Period</span>
                    <select value={popularPeriod} onChange={(event) => {
                      const nextPeriod = event.target.value as PopularPeriod;
                      setPopularPeriod(nextPeriod);
                      setPopularAnchorDate((current) => normalizePopularAnchor(current, nextPeriod));
                    }}>
                      <option value="Day">Day</option>
                      <option value="Week">Week</option>
                      <option value="Month">Month</option>
                    </select>
                  </label>
                  <button
                    className="button button-outlined"
                    type="button"
                    aria-label={`Earlier popular ${popularPeriod.toLowerCase()}`}
                    onClick={() => setPopularAnchorDate(shiftPopularAnchor(popularAnchorDate, popularPeriod, -1))}
                  >
                    Earlier
                  </button>
                  <label className="period-picker period-date">
                    <span>Date</span>
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
                    Later
                  </button>
                </div>
              )}
              {view === "search" && submittedSearch && (
                <button className="button button-outlined" type="button" onClick={() => void handleSaveQuery()}>Save query</button>
              )}
              <span className="content-policy">{contentPolicyLabel(activeContentPolicy)}</span>
              </>}
            </div>
          </section>

          {isBrowseView && selectionMode && (
            <div className="batch-toolbar" role="toolbar" aria-label="Batch download">
              <strong>{selectedPosts.length} selected</strong>
              <button className="button button-text" type="button" onClick={selectCurrentPage}>Select loaded posts</button>
              <button className="button button-primary" type="button" disabled={!selectedPosts.length || batchDownloading} onClick={() => void handleBatchDownload()}>
                {batchDownloading ? "Queueing…" : "Download selected"}
              </button>
            </div>
          )}

          {error && !queryError && <p className="message message-error" role="alert">{error}</p>}
          {notice && <p className="message message-success" role="status">{notice}</p>}
          {isBrowseView ? (
            <>
              {loading && <div className="loading-line" role="status"><span /> Finding something good…</div>}
              {queryError ? (
                <ErrorState message={queryError} onRetry={() => void imagesQuery.refetch()} />
              ) : !loading && images.length === 0 ? (
                <div className="empty-state">
                  <span className="empty-symbol" aria-hidden="true">✦</span>
                  <h3>No posts found</h3>
                  <p>Try a broader tag search or switch back to the latest feed.</p>
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
                  setError(`Could not open file: ${errorMessage(reason)}`);
                }
              }}
              onLoadMore={() => void downloadsQuery.fetchNextPage()}
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
              onSelectPost={setSelectedPost}
              onDownload={handleDownload}
              onTag={chooseTag}
              downloadingId={downloadingId}
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
            post={selectedPost}
            downloading={downloadingId === selectedPost.post.id}
            onClose={() => setSelectedPost(null)}
            onDownload={handleDownload}
            favorited={favoritePostIds.has(selectedPost.post.id)}
            onFavorite={handleFavorite}
            onTag={chooseTag}
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
      {toast && <Toast state={toast} onClose={() => setToast(null)} />}
    </div>
  );
}

interface ConfigInput {
  downloadPath: string;
  contentPolicy: ContentPolicy;
  network: NetworkPolicy;
}

interface DownloadInput {
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

interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div className="error-state" role="alert">
      <span className="error-symbol" aria-hidden="true">!</span>
      <div>
        <h3>Couldn’t load this feed</h3>
        <p>{message.replace("Failed to load images: ", "")}</p>
      </div>
      <button className="button button-outlined" type="button" onClick={onRetry}>Try again</button>
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
      <button className="icon-button" type="button" aria-label="Dismiss notification" onClick={onClose}><Icon name="close" /></button>
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

  if (!hasNext && !loading) return <div className="load-more-end">You’ve reached the end.</div>;
  return (
    <div className="load-more" ref={sentinel}>
      {loading ? <><span /> Loading more…</> : <button className="button button-outlined" type="button" onClick={requestMore}>Load more</button>}
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
          <img src={previewUrl} alt={`Post ${post.post.id}`} loading="lazy" />
        ) : (
          <span className="missing-preview">Preview unavailable</span>
        )}
        <span className="dimensions">{post.width ?? "?"}×{post.height ?? "?"}</span>
        <span className="rating-pill">{post.rating}</span>
        {selectionMode && (
          <button
            className="selection-toggle"
            type="button"
            aria-label={`${selected ? "Deselect" : "Select"} post ${post.post.id}`}
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
            {downloading ? "Saving…" : "Download"}
          </button>
        </div>
      </div>
    </article>
  );
}

interface PostInspectorProps {
  post: Post;
  downloading: boolean;
  favorited: boolean;
  onClose: () => void;
  onDownload: (post: Post) => Promise<void>;
  onFavorite: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
}

function PostInspector({ post, downloading, favorited, onClose, onDownload, onFavorite, onTag }: PostInspectorProps) {
  const originalUrl = post.full_url ?? post.sample_url ?? post.preview_url;
  return (
    <aside className="detail-panel shell-surface" aria-label="Post details">
      <div className="inspector-heading">
        <div>
          <p className="eyebrow">Post details</p>
          <h2>#{post.post.id}</h2>
        </div>
        <button className="icon-button" type="button" aria-label="Close details" onClick={onClose}><Icon name="close" /></button>
      </div>
      <div className="detail-preview">
        <DetailImage key={post.post.id} post={post} />
      </div>
      <div className="detail-summary">
        <span>Yande.re post #{post.post.id}</span>
        {originalUrl && <a href={originalUrl} target="_blank" rel="noreferrer">Open original</a>}
      </div>
      <div className="inspector-section">
        <span className="section-label">Tags</span>
        <div className="tag-list">
          {post.tags.map((tag) => <button key={tag} className="tag-chip" type="button" onClick={() => onTag(tag)}>{tag}</button>)}
        </div>
      </div>
      <dl className="metadata">
        <div><dt>Site</dt><dd>Yande.re</dd></div>
        <div><dt>Post ID</dt><dd>{post.post.id}</dd></div>
        <div><dt>Rating</dt><dd>{post.rating}</dd></div>
        <div><dt>Size</dt><dd>{post.width ?? "?"}×{post.height ?? "?"}</dd></div>
        <div><dt>Score</dt><dd>{post.score ?? "—"}</dd></div>
        <div><dt>File size</dt><dd>{post.file_size ? `${Math.round(post.file_size / 1024)} KB` : "—"}</dd></div>
      </dl>
      <p className="detail-helper">Tags and metadata come from the feed result. Yande does not expose a separate post-lookup operation in this release.</p>
      <button className="button button-outlined button-wide" type="button" onClick={() => void onFavorite(post)}>
        {favorited ? "Remove from Yande favorites" : "Add to Yande favorites"}
      </button>
      <button className="button button-primary button-wide" disabled={downloading} onClick={() => void onDownload(post)}>
        {downloading ? "Saving…" : "Download best quality"}
      </button>
    </aside>
  );
}

function DetailImage({ post }: { post: Post }) {
  const sources = [post.preview_url, post.sample_url, post.full_url].filter(
    (url, index, all): url is string => Boolean(url) && all.indexOf(url) === index,
  );
  const [sourceIndex, setSourceIndex] = useState(0);
  const source = sources[sourceIndex];

  if (!source) return <span>Preview unavailable</span>;
  return (
    <img
      src={source}
      alt={`Post ${post.post.id}`}
      loading="eager"
      onError={() => setSourceIndex((current) => current + 1)}
    />
  );
}

interface DownloadPanelProps {
  records: DownloadRecord[];
  historyHasNext: boolean;
  historyLoading: boolean;
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
  onOpen: (path: string) => Promise<void>;
  onLoadMore: () => void;
}

function DownloadPanel({ records, historyHasNext, historyLoading, onCancel, onRetry, onOpen, onLoadMore }: DownloadPanelProps) {
  const active = records.filter((record) => isActiveDownload(record.status));
  const history = records.filter((record) => !isActiveDownload(record.status));
  const groups = new Map<string, DownloadRecord[]>();
  for (const record of history) {
    const date = new Date(record.created_at_ms);
    const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
    groups.set(key, [...(groups.get(key) ?? []), record]);
  }
  const historyGroups = [...groups.entries()].sort(([a], [b]) => b.localeCompare(a));
  return (
    <section className="workspace-panel shell-surface" aria-label="Downloads">
      <div className="inspector-heading">
        <div><p className="eyebrow">Local state</p><h2>Downloads</h2></div>
      </div>
      <p className="helper-text">{active.length ? `${active.length} item${active.length === 1 ? "" : "s"} in progress` : "Nothing is downloading"}</p>
      {records.length === 0 ? (
        <div className="panel-empty">Your download history will appear here.</div>
      ) : (
        <>
          {active.length > 0 && <section className="download-section">
            <h3 className="download-section-title">In progress</h3>
            <div className="download-list">
              {active.map((record) => <DownloadRow key={record.id} record={record} onCancel={onCancel} onRetry={onRetry} onOpen={onOpen} />)}
            </div>
          </section>}
          {historyGroups.length > 0 && <section className="download-section">
            <h3 className="download-section-title">History</h3>
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
        {canCancel && <button className="button button-text" type="button" onClick={() => void onCancel(record.id)}>Cancel</button>}
        {canRetry && <button className="button button-outlined" type="button" onClick={() => void onRetry(record.id)}>Retry</button>}
        {canOpen && <button className="button button-outlined" type="button" onClick={() => void onOpen(record.target_path!)}>Open file</button>}
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
  onSelectPost: (post: Post) => void;
  onDownload: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
  downloadingId: string | null;
}

function PoolPanel({ pools, poolsLoading, poolsError, poolsHasNext, selectedPool, posts, postsLoading, postsError, postsHasNext, onRetryPools, onRetryPosts, onLoadMorePools, onLoadMorePosts, onBack, onBrowse, onSelectPost, onDownload, onTag, downloadingId }: PoolPanelProps) {
  if (selectedPool) {
    return (
      <section className="workspace-panel shell-surface" aria-label={`${selectedPool.name} pool`}>
        <div className="inspector-heading">
          <div><p className="eyebrow">Yande pool</p><h2>{selectedPool.name}</h2></div>
          <button className="button button-outlined button-with-icon" type="button" onClick={onBack}><Icon name="back" /><span>All pools</span></button>
        </div>
        <p className="helper-text">{selectedPool.post_count} ordered post{selectedPool.post_count === 1 ? "" : "s"} from Yande.</p>
        {postsError && <div className="panel-error" role="alert"><p>{postsError}</p><button className="button button-outlined" type="button" onClick={onRetryPosts}>Try again</button></div>}
        {postsLoading && posts.length === 0 && <p className="loading-line" role="status"><span /> Loading pool posts…</p>}
        {!postsLoading && !postsError && posts.length === 0 && <div className="panel-empty">This pool has no visible posts.</div>}
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
    <section className="workspace-panel shell-surface" aria-label="Pools">
      <div className="inspector-heading">
        <div><p className="eyebrow">Yande collections</p><h2>Pools</h2></div>
      </div>
      <p className="helper-text">Public pools group ordered posts from the site. ZIP download will be enabled after authenticated archive handling is wired.</p>
      {poolsLoading && pools.length === 0 && <p className="loading-line" role="status"><span /> Loading pools…</p>}
      {poolsError && <div className="panel-error" role="alert"><p>{poolsError}</p><button className="button button-outlined" type="button" onClick={onRetryPools}>Try again</button></div>}
      {!poolsLoading && !poolsError && pools.length === 0 && <div className="panel-empty">No public pools found.</div>}
      <div className="collection-list">
        {pools.map((pool) => (
          <article className="collection-row" key={pool.id}>
            <div><strong>{pool.name}</strong><p>{pool.post_count} post{pool.post_count === 1 ? "" : "s"}</p></div>
            <button className="button button-outlined" type="button" onClick={() => onBrowse(pool)}>Browse</button>
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
    <section className="workspace-panel shell-surface" aria-label="Favorites account">
      <div className="inspector-heading">
        <div><p className="eyebrow">Yande account</p><h2>Favorites</h2></div>
      </div>
      {auth?.authenticated ? (
        <>
          <p className="account-connected"><span className="connection-dot" /> {auth.username ? `${auth.username} connected` : "Yande account connected"}</p>
          {favoritesError && <div className="panel-error" role="alert"><p>{favoritesError}</p><button className="button button-outlined" type="button" onClick={onRetry}>Try again</button></div>}
          {favoritesLoading && favorites.length === 0 && <p className="helper-text">Loading favorites…</p>}
          {!favoritesLoading && favorites.length === 0 && <div className="panel-empty">No favorites found.</div>}
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
          <button className="button button-outlined" type="button" onClick={() => void onSignOut()}>Sign out</button>
        </>
      ) : (
        <>
          <p className="helper-text">Sign in through Yande’s own page. Dreamland reads only the safe auth state and keeps the browser session in Rust.</p>
          <button className="button button-primary button-wide" type="button" onClick={onBeginAuth}>Sign in to Yande</button>
          <button className="button button-outlined button-wide" type="button" disabled={loading} onClick={onRefresh}>{loading ? "Checking…" : "Check login"}</button>
        </>
      )}
    </section>
  );
}

function downloadStatusLabel(status: DownloadStatus): string {
  switch (status) {
    case "ExistingTarget": return "Already exists";
    case "Queued": return "Queued";
    case "Running": return "Downloading";
    case "Completed": return "Completed";
    case "Failed": return "Failed";
    case "Cancelled": return "Cancelled";
  }
}

interface SettingsDialogProps {
  config: AppConfig | null;
  onCancel: () => void;
  onSave: (downloadPath: string, contentPolicy: ContentPolicy, network: NetworkPolicy) => Promise<void>;
}

function SettingsDialog({ config, onCancel, onSave }: SettingsDialogProps) {
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
  const [contentPolicy, setContentPolicy] = useState<ContentPolicy>(config?.content_policy ?? "SafeOnly");
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
    await onSave(downloadPath, contentPolicy, { proxy, max_retries: maxRetries, retry_delay_ms: retryDelayMs, max_retry_delay_ms: maxRetryDelayMs });
    setSaving(false);
  }

  async function handleDetectProxy() {
    setDetecting(true);
    try {
      const result = await detectProxy();
      setProxyDetection(result.detected ? `Detected ${result.endpoint ?? "a system proxy"} (${result.source ?? "system"})` : "No proxy detected");
    } catch (reason) {
      setProxyDetection(`Detection failed: ${errorMessage(reason)}`);
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
          <div><p className="eyebrow">App preferences</p><h2 id="settings-title">Settings</h2></div>
          <button className="icon-button" type="button" aria-label="Close settings" onClick={onCancel}><Icon name="close" /></button>
        </div>
        <div className="settings-section">
          <p className="section-label">Storage & site</p>
          <label className="field">Download path<input required value={downloadPath} onChange={(event) => setDownloadPath(event.target.value)} /></label>
          <label className="field">Content policy
            <select value={contentPolicy} onChange={(event) => setContentPolicy(event.target.value as ContentPolicy)}>
              <option value="SafeOnly">Safe only (default)</option>
              <option value="AllowQuestionable">Allow questionable</option>
              <option value="AllowExplicit">Allow explicit</option>
              <option value="ExplicitOnly">Explicit only</option>
            </select>
          </label>
          <p className="helper-text">This controls which ratings appear in feeds and searches.</p>
        </div>
        <div className="settings-section">
          <p className="section-label">Connection resilience</p>
          <label className="field">Proxy
            <select value={proxyMode} onChange={(event) => setProxyMode(event.target.value as "Auto" | "Direct" | "Manual")}>
              <option value="Auto">Use system / environment</option>
              <option value="Direct">Direct connection</option>
              <option value="Manual">Manual proxy</option>
            </select>
          </label>
          {proxyMode === "Manual" && <label className="field">Proxy URL<input required placeholder="http://127.0.0.1:7890" value={proxyUrl} onChange={(event) => setProxyUrl(event.target.value)} /></label>}
          <button className="button button-outlined" type="button" disabled={detecting} onClick={() => void handleDetectProxy()}>{detecting ? "Detecting…" : "Detect proxy"}</button>
          {proxyDetection && <p className="helper-text" role="status">{proxyDetection}</p>}
          <div className="field-grid">
            <label className="field">Max retries<input type="number" min="0" max="8" value={maxRetries} onChange={(event) => setMaxRetries(Number(event.target.value))} /></label>
            <label className="field">Initial delay<input type="number" min="100" value={retryDelayMs} onChange={(event) => setRetryDelayMs(Number(event.target.value))} /></label>
          </div>
          <label className="field">Maximum retry delay<input type="number" min="100" value={maxRetryDelayMs} onChange={(event) => setMaxRetryDelayMs(Number(event.target.value))} /></label>
          <p className="helper-text">429 and temporary server responses retry with a bounded delay.</p>
        </div>
        <div className="dialog-actions">
          <button className="button button-text" type="button" onClick={onCancel}>Cancel</button>
          <button className="button button-primary" type="submit" disabled={saving}>{saving ? "Saving…" : "Save settings"}</button>
        </div>
      </form>
    </div>
  );
}

export default App;

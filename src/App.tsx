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
  cancelDownload,
  retryDownload,
  loadConfig,
  queryPosts,
  saveConfig,
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
} from "./lib/ipc";

type ViewMode = "latest" | "popular" | "search" | "downloads" | "pools" | "favorites";
type PopularPeriod = "Day" | "Week" | "Month";
type ToastTone = "success" | "info" | "error";

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
  return new Date().toISOString().slice(0, 10);
}

function contentPolicyLabel(policy: ContentPolicy): string {
  switch (policy) {
    case "SafeOnly": return "Safe only";
    case "AllowQuestionable": return "Safe + questionable";
    case "AllowExplicit": return "All ratings";
    case "ExplicitOnly": return "Explicit only";
  }
}

function App() {
  const queryClient = useQueryClient();
  const [view, setView] = useState<ViewMode>("latest");
  const [popularPeriod, setPopularPeriod] = useState<PopularPeriod>("Week");
  const [searchDraft, setSearchDraft] = useState("");
  const [submittedSearch, setSubmittedSearch] = useState("");
  const [searchFocused, setSearchFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [poolPage, setPoolPage] = useState(1);
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
  const pageSize = configQuery.data?.images_per_page ?? 20;
  const contentPolicy = configQuery.data?.content_policy ?? "SafeOnly";
  const request = useMemo<PostQueryRequest>(() => {
    let source: DiscoverySource = "Browse";
    if (view === "popular") {
      source = {
        Feed: {
          kind: {
            Popular: { period: popularPeriod, anchor_date: today() },
          },
        },
      };
    } else if (view === "search" && submittedSearch) {
      source = { Search: { expression: submittedSearch } };
    }

    return {
      query: { source, content_policy: contentPolicy },
      pagination: view === "popular"
        ? "FixedWindow"
        : { First: { page_size: pageSize } },
    };
  }, [contentPolicy, pageSize, popularPeriod, submittedSearch, view]);
  const imagesQuery = useInfiniteQuery({
    queryKey: ["posts", request],
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) => pageParam ? continueQuery(pageParam) : queryPosts(request),
    getNextPageParam: (lastPage) => view !== "popular" && lastPage.session && lastPage.posts.length >= lastPage.page_size
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
  const downloadsQuery = useQuery({
    queryKey: ["downloads"],
    queryFn: () => listDownloads(50),
    enabled: configQuery.isSuccess,
    refetchInterval: 1_500,
  });
  useEffect(() => {
    if (!downloadsQuery.data) return;
    for (const record of downloadsQuery.data) {
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
    enabled: searchFocused && searchDraft.trim().length >= 2,
    staleTime: 30_000,
  });
  const poolsQuery = useQuery({
    queryKey: ["pools", poolPage],
    queryFn: () => listPools(poolPage, 30),
    enabled: view === "pools",
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
  const selectedPosts = images.filter((post) => selectedPostIds.has(post.post.id));
  const loading = configQuery.isPending || imagesQuery.isPending;
  const loadingMore = imagesQuery.isFetchingNextPage;
  const downloadRecords = downloadsQuery.data ?? [];
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
      ? `Most popular this ${popularPeriod.toLowerCase()}`
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
    if (nextView === "pools") setPoolPage(1);
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
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
  }

  function chooseTag(tag: string) {
    setSearchDraft(tag);
    setSubmittedSearch(tag);
    setView("search");
    setSearchFocused(false);
    setSelectedPostIds(new Set());
    setSelectionMode(false);
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
          <span className="search-icon" aria-hidden="true">⌕</span>
          <input
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
            ↻
          </button>
          <button className="button button-tonal" type="button" onClick={() => setSettingsOpen(true)}>
            Settings
          </button>
        </div>
      </header>

      <div className={`app-layout${selectedPost ? " has-detail" : ""}`}>
        <main className="content">
          <nav className="view-tabs shell-surface" aria-label="Dreamland sections" role="tablist">
          <NavButton active={view === "latest"} label="Latest" icon="◷" onClick={() => changeView("latest")} />
          <NavButton active={view === "popular"} label="Popular" icon="↗" onClick={() => changeView("popular")} />
          <NavButton active={view === "downloads"} label="Downloads" icon="⇩" onClick={() => changeView("downloads")} />
          <NavButton active={view === "pools"} label="Pools" icon="▦" onClick={() => changeView("pools")} />
          <NavButton active={view === "favorites"} label="Favorites" icon="♡" onClick={() => changeView("favorites")} />
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
                <label className="period-picker">
                  <span>Period</span>
                  <select value={popularPeriod} onChange={(event) => {
                    setPopularPeriod(event.target.value as PopularPeriod);
                  }}>
                    <option value="Day">Day</option>
                    <option value="Week">Week</option>
                    <option value="Month">Month</option>
                  </select>
                  </label>
              )}
              <span className="content-policy">{contentPolicyLabel(contentPolicy)}</span>
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
                  {view !== "popular" && <LoadMore
                    hasNext={Boolean(imagesQuery.hasNextPage)}
                    loading={loadingMore}
                    onLoadMore={() => void imagesQuery.fetchNextPage()}
                  />}
                </>
              )}
            </>
          ) : view === "downloads" ? (
            <DownloadPanel
              records={downloadRecords}
              onCancel={async (id) => {
                await cancelDownload(id);
                await queryClient.invalidateQueries({ queryKey: ["downloads"] });
              }}
              onRetry={async (id) => {
                await retryDownload(id);
                await queryClient.invalidateQueries({ queryKey: ["downloads"] });
              }}
            />
          ) : view === "pools" ? (
            <PoolPanel
              pools={poolsQuery.data?.pools ?? []}
              page={poolPage}
              hasNext={poolsQuery.data?.has_next ?? false}
              loading={poolsQuery.isPending || poolsQuery.isFetching}
              error={poolsQuery.error ? errorMessage(poolsQuery.error) : ""}
              onRetry={() => void poolsQuery.refetch()}
              onPrevious={() => setPoolPage((current) => Math.max(1, current - 1))}
              onNext={() => setPoolPage((current) => current + 1)}
              onBrowse={(pool) => chooseTag(`pool:${pool.id}`)}
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
  icon: string;
  label: string;
  onClick?: () => void;
}

function NavButton({ active, disabled, hint, icon, label, onClick }: NavButtonProps) {
  return (
    <button className={`nav-item${active ? " active" : ""}`} disabled={disabled} onClick={onClick} title={hint} role="tab" aria-selected={active}>
      <span className="nav-icon" aria-hidden="true">{icon}</span>
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
      <button className="icon-button" type="button" aria-label="Dismiss notification" onClick={onClose}>×</button>
    </aside>
  );
}

interface LoadMoreProps {
  hasNext: boolean;
  loading: boolean;
  onLoadMore: () => void;
}

function LoadMore({ hasNext, loading, onLoadMore }: LoadMoreProps) {
  const sentinel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!hasNext || loading || !sentinel.current) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting) onLoadMore();
    }, { rootMargin: "800px" });
    observer.observe(sentinel.current);
    return () => observer.disconnect();
  }, [hasNext, loading, onLoadMore]);

  if (!hasNext && !loading) return <div className="load-more-end">You’ve reached the end.</div>;
  return (
    <div className="load-more" ref={sentinel}>
      {loading ? <><span /> Loading more…</> : <button className="button button-outlined" type="button" onClick={onLoadMore}>Load more</button>}
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
            {selected ? "✓" : ""}
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
        <button className="icon-button" type="button" aria-label="Close details" onClick={onClose}>×</button>
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
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
}

function DownloadPanel({ records, onCancel, onRetry }: DownloadPanelProps) {
  const active = records.filter((record) => record.status === "Queued" || record.status === "Running");
  return (
    <section className="workspace-panel shell-surface" aria-label="Downloads">
      <div className="inspector-heading">
        <div><p className="eyebrow">Local state</p><h2>Downloads</h2></div>
      </div>
      <p className="helper-text">{active.length ? `${active.length} item${active.length === 1 ? "" : "s"} in progress` : "Nothing is downloading"}</p>
      {records.length === 0 ? (
        <div className="panel-empty">Your download history will appear here.</div>
      ) : (
        <div className="download-list">
          {records.map((record) => <DownloadRow key={record.id} record={record} onCancel={onCancel} onRetry={onRetry} />)}
        </div>
      )}
    </section>
  );
}

interface DownloadRowProps {
  record: DownloadRecord;
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
}

function DownloadRow({ record, onCancel, onRetry }: DownloadRowProps) {
  const canCancel = record.status === "Queued" || record.status === "Running";
  const canRetry = record.status === "Failed" || record.status === "Cancelled";
  return (
    <article className="download-row">
      <div className="download-row-heading">
        <strong>#{record.post_id}</strong>
        <span className={`download-status status-${record.status.toLowerCase()}`}>{downloadStatusLabel(record.status)}</span>
      </div>
      <p>{record.variant} quality · attempt {record.attempts || 1}</p>
      {record.error && <p className="download-error">{record.error}</p>}
      {record.target_path && <p className="download-path" title={record.target_path}>{record.target_path}</p>}
      {(canCancel || canRetry) && <div className="download-row-actions">
        {canCancel && <button className="button button-text" type="button" onClick={() => void onCancel(record.id)}>Cancel</button>}
        {canRetry && <button className="button button-outlined" type="button" onClick={() => void onRetry(record.id)}>Retry</button>}
      </div>}
    </article>
  );
}

interface PoolPanelProps {
  pools: Pool[];
  page: number;
  hasNext: boolean;
  loading: boolean;
  error: string;
  onRetry: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onBrowse: (pool: Pool) => void;
}

function PoolPanel({ pools, page, hasNext, loading, error, onRetry, onPrevious, onNext, onBrowse }: PoolPanelProps) {
  return (
    <section className="workspace-panel shell-surface" aria-label="Pools">
      <div className="inspector-heading">
        <div><p className="eyebrow">Yande collections</p><h2>Pools</h2></div>
      </div>
      <p className="helper-text">Public pools group ordered posts from the site. ZIP download will be enabled after authenticated archive handling is wired.</p>
      {loading && <p className="helper-text">Loading pools…</p>}
      {error && <div className="panel-error" role="alert"><p>{error}</p><button className="button button-outlined" type="button" onClick={onRetry}>Try again</button></div>}
      {!loading && !error && pools.filter((pool) => pool.public).length === 0 && <div className="panel-empty">No public pools found.</div>}
      <div className="collection-list">
        {pools.filter((pool) => pool.public).map((pool) => (
          <article className="collection-row" key={pool.id}>
            <div><strong>{pool.name}</strong><p>{pool.post_count} post{pool.post_count === 1 ? "" : "s"}</p></div>
            <button className="button button-outlined" type="button" onClick={() => onBrowse(pool)}>Browse</button>
          </article>
        ))}
      </div>
      {!loading && !error && (
        <div className="collection-pagination">
          <button className="button button-outlined" type="button" disabled={page <= 1} onClick={onPrevious}>Previous</button>
          <span>Page {page}</span>
          <button className="button button-outlined" type="button" disabled={!hasNext} onClick={onNext}>Next</button>
        </div>
      )}
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
          <button className="icon-button" type="button" aria-label="Close settings" onClick={onCancel}>×</button>
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

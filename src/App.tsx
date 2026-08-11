import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  detectProxy,
  enqueueDownload,
  listDownloads,
  cancelDownload,
  retryDownload,
  loadConfig,
  queryPosts,
  saveConfig,
  suggestTags,
  type AppConfig,
  type ContentPolicy,
  type DiscoverySource,
  type DownloadRecord,
  type DownloadStatus,
  type MediaVariant,
  type NetworkPolicy,
  type Post,
  type PostQueryRequest,
  type ProxyMode,
} from "./lib/ipc";

type ViewMode = "latest" | "popular" | "search";
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
  const [page, setPage] = useState(1);
  const [view, setView] = useState<ViewMode>("latest");
  const [popularPeriod, setPopularPeriod] = useState<PopularPeriod>("Week");
  const [searchDraft, setSearchDraft] = useState("");
  const [submittedSearch, setSubmittedSearch] = useState("");
  const [searchFocused, setSearchFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [downloadsOpen, setDownloadsOpen] = useState(false);
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [toast, setToast] = useState<ToastState | null>(null);
  const toastId = useRef(0);
  const downloadStatuses = useRef(new Map<string, DownloadStatus>());

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
        : { Page: { number: page, page_size: pageSize } },
    };
  }, [contentPolicy, page, pageSize, popularPeriod, submittedSearch, view]);
  const imagesQuery = useQuery({
    queryKey: ["posts", request],
    queryFn: () => queryPosts(request),
    enabled: configQuery.isSuccess,
  });
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

  const queryError = configQuery.error
    ? `Failed to load configuration: ${errorMessage(configQuery.error)}`
    : imagesQuery.error
      ? `Failed to load images: ${errorMessage(imagesQuery.error)}`
      : "";
  const images = imagesQuery.data?.posts ?? [];
  const loading = configQuery.isPending || imagesQuery.isFetching;
  const downloadRecords = downloadsQuery.data ?? [];
  const title = view === "search" ? "Search results" : view === "popular" ? "Popular" : "Latest posts";
  const subtitle = view === "search"
    ? `Matching “${submittedSearch}”`
    : view === "popular"
      ? `Most popular this ${popularPeriod.toLowerCase()}`
      : "A calm feed for finding something worth keeping";

  async function requestPage(nextPage: number) {
    setError("");
    setNotice("");
    setPage(nextPage);
    if (nextPage === page) {
      await imagesQuery.refetch();
    }
  }

  function changeView(nextView: ViewMode) {
    setError("");
    setNotice("");
    setPage(1);
    setView(nextView);
    setDownloadsOpen(false);
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
    setPage(1);
    setSubmittedSearch(expression);
    setView("search");
    setSearchFocused(false);
  }

  function chooseTag(tag: string) {
    setSearchDraft(tag);
    setSubmittedSearch(tag);
    setPage(1);
    setView("search");
    setSearchFocused(false);
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
            aria-label="Refresh posts"
            title="Refresh posts"
            disabled={loading}
            onClick={() => void imagesQuery.refetch()}
          >
            ↻
          </button>
          <button className="button button-tonal" type="button" onClick={() => setSettingsOpen(true)}>
            Settings
          </button>
        </div>
      </header>

      <div className={`app-layout${selectedPost ? " has-detail" : ""}`}>
        <nav className="nav-rail shell-surface" aria-label="Explore">
          <p className="nav-heading">Explore</p>
          <NavButton active={view === "latest"} label="Latest" icon="◷" onClick={() => changeView("latest")} />
          <NavButton active={view === "popular"} label="Popular" icon="↗" onClick={() => changeView("popular")} />
          <p className="nav-heading nav-heading-spaced">Your space</p>
          <NavButton active={downloadsOpen} label="Downloads" icon="⇩" onClick={() => setDownloadsOpen(true)} />
          <NavButton disabled label="Pools" icon="▦" hint="Gated" />
          <NavButton disabled label="Favorites" icon="♡" hint="Gated" />
          <div className="nav-footer">
            <span className="connection-dot" aria-hidden="true" />
            <span>Yandere connected</span>
          </div>
        </nav>

        <main className="content">
          <section className="content-heading">
            <div>
              <p className="eyebrow">Explore freely</p>
              <h2>{title}</h2>
              <p className="subtitle">{subtitle}</p>
            </div>
            <div className="heading-actions">
              {view === "popular" && (
                <label className="period-picker">
                  <span>Period</span>
                  <select value={popularPeriod} onChange={(event) => {
                    setPopularPeriod(event.target.value as PopularPeriod);
                    setPage(1);
                  }}>
                    <option value="Day">Day</option>
                    <option value="Week">Week</option>
                    <option value="Month">Month</option>
                  </select>
                </label>
              )}
              <span className="content-policy">{contentPolicyLabel(contentPolicy)}</span>
            </div>
          </section>

          {error && !queryError && <p className="message message-error" role="alert">{error}</p>}
          {notice && <p className="message message-success" role="status">{notice}</p>}
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
                    downloading={downloadingId === post.post.id}
                    onDownload={handleDownload}
                    onSelect={setSelectedPost}
                    onTag={chooseTag}
                  />
                ))}
              </div>
              {view !== "popular" && (
                <div className="pagination">
                  <button className="button button-outlined" disabled={loading || page <= 1} onClick={() => void requestPage(page - 1)}>
                    Previous
                  </button>
                  <span>Page {page}</span>
                  <button className="button button-outlined" disabled={loading || images.length < pageSize} onClick={() => void requestPage(page + 1)}>
                    Next
                  </button>
                </div>
              )}
            </>
          )}
        </main>

        {selectedPost && (
          <PostInspector
            post={selectedPost}
            downloading={downloadingId === selectedPost.post.id}
            onClose={() => setSelectedPost(null)}
            onDownload={handleDownload}
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
      {downloadsOpen && (
        <DownloadPanel
          records={downloadRecords}
          onClose={() => setDownloadsOpen(false)}
          onCancel={async (id) => {
            await cancelDownload(id);
            await queryClient.invalidateQueries({ queryKey: ["downloads"] });
          }}
          onRetry={async (id) => {
            await retryDownload(id);
            await queryClient.invalidateQueries({ queryKey: ["downloads"] });
          }}
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
    <button className={`nav-item${active ? " active" : ""}`} disabled={disabled} onClick={onClick} title={hint}>
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

interface ImageCardProps {
  post: Post;
  downloading: boolean;
  onDownload: (post: Post) => Promise<void>;
  onSelect: (post: Post) => void;
  onTag: (tag: string) => void;
}

function ImageCard({ post, downloading, onDownload, onSelect, onTag }: ImageCardProps) {
  const previewUrl = post.preview_url ?? post.sample_url ?? post.full_url;
  return (
    <article className="card" onClick={() => onSelect(post)}>
      <div className="preview">
        {previewUrl ? (
          <img src={previewUrl} alt={`Post ${post.post.id}`} loading="lazy" />
        ) : (
          <span className="missing-preview">Preview unavailable</span>
        )}
        <span className="dimensions">{post.width ?? "?"}×{post.height ?? "?"}</span>
        <span className="rating-pill">{post.rating}</span>
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
  onClose: () => void;
  onDownload: (post: Post) => Promise<void>;
  onTag: (tag: string) => void;
}

function PostInspector({ post, downloading, onClose, onDownload, onTag }: PostInspectorProps) {
  const previewUrl = post.sample_url ?? post.full_url ?? post.preview_url;
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
        {previewUrl ? <img src={previewUrl} alt={`Post ${post.post.id}`} /> : <span>Preview unavailable</span>}
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
      <button className="button button-primary button-wide" disabled={downloading} onClick={() => void onDownload(post)}>
        {downloading ? "Saving…" : "Download best quality"}
      </button>
    </aside>
  );
}

interface DownloadPanelProps {
  records: DownloadRecord[];
  onClose: () => void;
  onCancel: (id: string) => Promise<void>;
  onRetry: (id: string) => Promise<void>;
}

function DownloadPanel({ records, onClose, onCancel, onRetry }: DownloadPanelProps) {
  const active = records.filter((record) => record.status === "Queued" || record.status === "Running");
  return (
    <aside className="download-panel shell-surface" aria-label="Downloads">
      <div className="inspector-heading">
        <div><p className="eyebrow">Local state</p><h2>Downloads</h2></div>
        <button className="icon-button" type="button" aria-label="Close downloads" onClick={onClose}>×</button>
      </div>
      <p className="helper-text">{active.length ? `${active.length} item${active.length === 1 ? "" : "s"} in progress` : "Nothing is downloading"}</p>
      {records.length === 0 ? (
        <div className="panel-empty">Your download history will appear here.</div>
      ) : (
        <div className="download-list">
          {records.map((record) => <DownloadRow key={record.id} record={record} onCancel={onCancel} onRetry={onRetry} />)}
        </div>
      )}
    </aside>
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

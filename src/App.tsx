import { useMemo, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  detectProxy,
  downloadImage,
  loadConfig,
  queryPosts,
  saveConfig,
  suggestTags,
  type AppConfig,
  type DiscoverySource,
  type MediaVariant,
  type NetworkPolicy,
  type Post,
  type PostQueryRequest,
  type ProxyMode,
} from "./lib/ipc";

type ViewMode = "latest" | "popular" | "search";
type PopularPeriod = "Day" | "Week" | "Month";

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
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
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const configQuery = useQuery({
    queryKey: ["config"],
    queryFn: loadConfig,
  });
  const pageSize = configQuery.data?.images_per_page ?? 20;
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
      query: { source, content_policy: "SafeOnly" },
      pagination: { Page: { number: page, page_size: pageSize } },
    };
  }, [page, pageSize, popularPeriod, submittedSearch, view]);
  const imagesQuery = useQuery({
    queryKey: ["posts", request],
    queryFn: () => queryPosts(request),
    enabled: configQuery.isSuccess,
  });
  const suggestionsQuery = useQuery({
    queryKey: ["tag-suggestions", searchDraft.trim()],
    queryFn: () => suggestTags(searchDraft.trim(), 7),
    enabled: searchFocused && searchDraft.trim().length >= 2,
    staleTime: 30_000,
  });
  const saveConfigMutation = useMutation({
    mutationFn: ({ downloadPath, apiUrl, network }: ConfigInput) =>
      saveConfig(downloadPath, apiUrl, network),
    onSuccess: (nextConfig) => {
      queryClient.setQueryData(["config"], nextConfig);
      setSettingsOpen(false);
      setNotice("Settings saved");
    },
    onError: (reason) => setError(`Failed to save settings: ${errorMessage(reason)}`),
  });
  const downloadMutation = useMutation({
    mutationFn: ({ postId, variant }: DownloadInput) => downloadImage(postId, variant),
    onSuccess: (path) => setNotice(`Downloaded to ${path}`),
    onError: (reason) => setError(`Download failed: ${errorMessage(reason)}`),
  });

  const queryError = configQuery.error
    ? `Failed to load configuration: ${errorMessage(configQuery.error)}`
    : imagesQuery.error
      ? `Failed to load images: ${errorMessage(imagesQuery.error)}`
      : "";
  const images = imagesQuery.data?.posts ?? [];
  const loading = configQuery.isPending || imagesQuery.isFetching;
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
    apiUrl: string,
    network: NetworkPolicy,
  ) {
    setError("");
    setNotice("");
    try {
      await saveConfigMutation.mutateAsync({ downloadPath, apiUrl, network });
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
              <span className="content-policy">Safe by default</span>
            </div>
          </section>

          {(error || queryError) && <p className="message message-error" role="alert">{error || queryError}</p>}
          {notice && <p className="message message-success" role="status">{notice}</p>}
          {loading && <div className="loading-line" role="status"><span /> Finding something good…</div>}
          {!loading && images.length === 0 ? (
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
              <div className="pagination">
                <button className="button button-outlined" disabled={loading || page <= 1} onClick={() => void requestPage(page - 1)}>
                  Previous
                </button>
                <span>Page {page}</span>
                <button className="button button-outlined" disabled={loading || images.length < pageSize} onClick={() => void requestPage(page + 1)}>
                  Next
                </button>
              </div>
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
    </div>
  );
}

interface ConfigInput {
  downloadPath: string;
  apiUrl: string;
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
      <div className="inspector-section">
        <span className="section-label">Tags</span>
        <div className="tag-list">
          {post.tags.map((tag) => <button key={tag} className="tag-chip" type="button" onClick={() => onTag(tag)}>{tag}</button>)}
        </div>
      </div>
      <dl className="metadata">
        <div><dt>Rating</dt><dd>{post.rating}</dd></div>
        <div><dt>Size</dt><dd>{post.width ?? "?"}×{post.height ?? "?"}</dd></div>
        <div><dt>Score</dt><dd>{post.score ?? "—"}</dd></div>
        <div><dt>File size</dt><dd>{post.file_size ? `${Math.round(post.file_size / 1024)} KB` : "—"}</dd></div>
      </dl>
      <button className="button button-primary button-wide" disabled={downloading} onClick={() => void onDownload(post)}>
        {downloading ? "Saving…" : "Download best quality"}
      </button>
    </aside>
  );
}

interface SettingsDialogProps {
  config: AppConfig | null;
  onCancel: () => void;
  onSave: (downloadPath: string, apiUrl: string, network: NetworkPolicy) => Promise<void>;
}

function SettingsDialog({ config, onCancel, onSave }: SettingsDialogProps) {
  const [apiUrl, setApiUrl] = useState(config?.api_url ?? "");
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
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
    await onSave(downloadPath, apiUrl, { proxy, max_retries: maxRetries, retry_delay_ms: retryDelayMs, max_retry_delay_ms: maxRetryDelayMs });
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
          <label className="field">API URL<input required value={apiUrl} onChange={(event) => setApiUrl(event.target.value)} /></label>
          <label className="field">Download path<input required value={downloadPath} onChange={(event) => setDownloadPath(event.target.value)} /></label>
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

import { useState } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  downloadImage,
  loadConfig,
  loadImages,
  saveConfig,
  type AppConfig,
  type ImagePost,
} from "./lib/ipc";

const defaultApiUrl = "https://yande.re/post.json";

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function App() {
  const queryClient = useQueryClient();
  const [page, setPage] = useState(1);
  const [imagesRequested, setImagesRequested] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [downloadingId, setDownloadingId] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const configQuery = useQuery({
    queryKey: ["config"],
    queryFn: loadConfig,
  });
  const imagesQuery = useQuery({
    queryKey: ["images", page],
    queryFn: () => loadImages(page),
    enabled: configQuery.isSuccess && imagesRequested,
  });
  const saveConfigMutation = useMutation({
    mutationFn: ({ downloadPath, apiUrl }: ConfigInput) =>
      saveConfig(downloadPath, apiUrl),
    onSuccess: (nextConfig) => {
      queryClient.setQueryData(["config"], nextConfig);
      setSettingsOpen(false);
      setNotice("Settings saved");
    },
    onError: (reason) => setError(`Failed to save settings: ${errorMessage(reason)}`),
  });
  const downloadMutation = useMutation({
    mutationFn: downloadImage,
    onSuccess: (path) => setNotice(`Downloaded to ${path}`),
    onError: (reason) => setError(`Download failed: ${errorMessage(reason)}`),
  });

  const queryError = configQuery.error
    ? `Failed to load configuration: ${errorMessage(configQuery.error)}`
    : imagesQuery.error
      ? `Failed to load images: ${errorMessage(imagesQuery.error)}`
      : "";
  const images = imagesQuery.data ?? [];
  const loading = configQuery.isPending || imagesQuery.isFetching;

  async function requestImages(nextPage: number) {
    setError("");
    setNotice("");
    setImagesRequested(true);
    if (nextPage === page) {
      await imagesQuery.refetch();
      return;
    }
    setPage(nextPage);
  }

  async function handleDownload(image: ImagePost) {
    setDownloadingId(image.id);
    setError("");
    setNotice("");
    try {
      await downloadMutation.mutateAsync(image);
    } catch {
      // The mutation reports the user-facing error.
    } finally {
      setDownloadingId(null);
    }
  }

  async function handleSaveConfig(downloadPath: string, apiUrl: string) {
    setError("");
    setNotice("");
    try {
      await saveConfigMutation.mutateAsync({ downloadPath, apiUrl });
    } catch {
      // The mutation reports the user-facing error.
    }
  }

  return (
    <div className="app">
      <header className="toolbar">
        <h1>Dreamland Image Viewer</h1>
        <div className="toolbar-actions">
          <button
            className="button button-success"
            disabled={loading}
            onClick={() => void requestImages(page)}
          >
            {loading ? "Loading..." : "Load Images"}
          </button>
          <button
            className="button button-primary"
            onClick={() => setSettingsOpen(true)}
          >
            Settings
          </button>
        </div>
      </header>

      <main className="content">
        {(error || queryError) && (
          <p className="message message-error">{error || queryError}</p>
        )}
        {notice && <p className="message message-success">{notice}</p>}
        {images.length === 0 ? (
          <p className="message">Click 'Load Images' to fetch images from the API</p>
        ) : (
          <>
            <div className="gallery-grid">
              {images.map((image) => (
                <ImageCard
                  key={image.id}
                  image={image}
                  downloading={downloadingId === image.id}
                  onDownload={handleDownload}
                />
              ))}
            </div>
            <div className="pagination">
              <button
                className="button button-primary"
                disabled={loading || page <= 1}
                onClick={() => void requestImages(page - 1)}
              >
                Previous
              </button>
              <strong>Page {page}</strong>
              <button
                className="button button-primary"
                disabled={loading}
                onClick={() => void requestImages(page + 1)}
              >
                Next
              </button>
            </div>
          </>
        )}
      </main>

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
}

interface ImageCardProps {
  image: ImagePost;
  downloading: boolean;
  onDownload: (image: ImagePost) => Promise<void>;
}

function ImageCard({ image, downloading, onDownload }: ImageCardProps) {
  return (
    <article className="card">
      <div className="preview">
        <img src={image.preview_url} alt="Image preview" loading="lazy" />
        <span className="dimensions">
          {image.width}x{image.height}
        </span>
      </div>
      <div className="card-details">
        <p className="tags">Tags: {image.tags}</p>
        <div className="card-footer">
          <span className="rating">Rating: {image.rating}</span>
          <button
            className="button button-warning"
            disabled={downloading}
            onClick={() => void onDownload(image)}
          >
            {downloading ? "Saving..." : "Download"}
          </button>
        </div>
      </div>
    </article>
  );
}

interface SettingsDialogProps {
  config: AppConfig | null;
  onCancel: () => void;
  onSave: (downloadPath: string, apiUrl: string) => Promise<void>;
}

function SettingsDialog({ config, onCancel, onSave }: SettingsDialogProps) {
  const [apiUrl, setApiUrl] = useState(config?.api_url ?? defaultApiUrl);
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
  const [saving, setSaving] = useState(false);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    await onSave(downloadPath, apiUrl);
    setSaving(false);
  }

  return (
    <div className="settings-overlay">
      <form className="settings-dialog" onSubmit={submit}>
        <h2>Settings</h2>
        <label>
          API URL
          <input
            required
            value={apiUrl}
            onChange={(event) => setApiUrl(event.target.value)}
          />
        </label>
        <label>
          Download Path
          <input
            required
            value={downloadPath}
            onChange={(event) => setDownloadPath(event.target.value)}
          />
        </label>
        <div className="dialog-actions">
          <button className="button button-muted" type="button" onClick={onCancel}>
            Cancel
          </button>
          <button className="button button-success" type="submit" disabled={saving}>
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </form>
    </div>
  );
}

export default App;

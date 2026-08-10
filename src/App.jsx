import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const defaultApiUrl = "https://yande.re/post.json";

function App() {
  const [config, setConfig] = useState(null);
  const [images, setImages] = useState([]);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [downloadingId, setDownloadingId] = useState(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  useEffect(() => {
    invoke("load_config")
      .then(setConfig)
      .catch((reason) => setError(`Failed to load configuration: ${reason}`));
  }, []);

  async function loadImages(nextPage) {
    setLoading(true);
    setError("");
    setNotice("");
    try {
      const result = await invoke("load_images", { page: nextPage });
      setImages(result);
      setPage(nextPage);
    } catch (reason) {
      setError(`Failed to load images: ${reason}`);
    } finally {
      setLoading(false);
    }
  }

  async function downloadImage(image) {
    setDownloadingId(image.id);
    setError("");
    setNotice("");
    try {
      const path = await invoke("download_image", { image });
      setNotice(`Downloaded to ${path}`);
    } catch (reason) {
      setError(`Download failed: ${reason}`);
    } finally {
      setDownloadingId(null);
    }
  }

  async function saveConfig(downloadPath, apiUrl) {
    try {
      const nextConfig = await invoke("save_config", { downloadPath, apiUrl });
      setConfig(nextConfig);
      setSettingsOpen(false);
      setNotice("Settings saved");
    } catch (reason) {
      setError(`Failed to save settings: ${reason}`);
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
            onClick={() => loadImages(page)}
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
        {error && <p className="message message-error">{error}</p>}
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
                  onDownload={downloadImage}
                />
              ))}
            </div>
            <div className="pagination">
              <button
                className="button button-primary"
                disabled={loading || page <= 1}
                onClick={() => loadImages(page - 1)}
              >
                Previous
              </button>
              <strong>Page {page}</strong>
              <button
                className="button button-primary"
                disabled={loading}
                onClick={() => loadImages(page + 1)}
              >
                Next
              </button>
            </div>
          </>
        )}
      </main>

      {settingsOpen && (
        <SettingsDialog
          config={config}
          onCancel={() => setSettingsOpen(false)}
          onSave={saveConfig}
        />
      )}
    </div>
  );
}

function ImageCard({ image, downloading, onDownload }) {
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
            onClick={() => onDownload(image)}
          >
            {downloading ? "Saving..." : "Download"}
          </button>
        </div>
      </div>
    </article>
  );
}

function SettingsDialog({ config, onCancel, onSave }) {
  const [apiUrl, setApiUrl] = useState(config?.api_url ?? defaultApiUrl);
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
  const [saving, setSaving] = useState(false);

  async function submit(event) {
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

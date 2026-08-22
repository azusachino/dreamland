import { useState, type FormEvent } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Snackbar,
  TextField,
} from "@mui/material";
import {
  type AppConfig,
  type ContentPolicy,
  type MediaVariant,
  type NetworkPolicy,
  type ProxyMode,
} from "../lib/ipc";
import { Icon } from "./Icon";

export type ThemeMode = "system" | "light" | "dark";
export type ToastTone = "success" | "info" | "error";

export interface ToastState {
  id: number;
  title: string;
  message: string;
  tone: ToastTone;
}

export type QueryOrder = "" | "score" | "score_asc" | "id" | "id_desc" | "mpixels" | "mpixels_asc" | "landscape" | "portrait" | "vote" | "random";

export interface AdvancedQueryForm {
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

interface AdvancedQueryDialogProps {
  contentPolicy: ContentPolicy;
  initialExpression: string;
  onClose: () => void;
  onApply: (expression: string) => void;
  parseQuery: (expression: string) => AdvancedQueryForm;
  buildQuery: (form: AdvancedQueryForm, contentPolicy: ContentPolicy) => string;
  today: () => string;
  formatError: (reason: unknown) => string;
}

export function AdvancedQueryDialog({ contentPolicy, initialExpression, onClose, onApply, parseQuery, buildQuery, today, formatError }: AdvancedQueryDialogProps) {
  const [form, setForm] = useState<AdvancedQueryForm>(() => parseQuery(initialExpression));
  const [error, setError] = useState("");

  function update<K extends keyof AdvancedQueryForm>(key: K, value: AdvancedQueryForm[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      onApply(buildQuery(form, contentPolicy));
    } catch (reason) {
      setError(formatError(reason));
    }
  }

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="md" className="query-dialog">
      <form onSubmit={submit}>
        <DialogTitle>
          <p className="eyebrow">moebooru query vocabulary</p>
          advanced search
          <IconButton type="button" aria-label="close advanced search" onClick={onClose} sx={{ position: "absolute", right: 12, top: 12 }}><Icon name="close" /></IconButton>
        </DialogTitle>
        <DialogContent dividers>
          <p className="helper-text">build or edit a reusable tag query with the same filters supported by the vendored moebooru client. unknown terms stay in the raw tag field.</p>
          {error && <Alert severity="error">{error}</Alert>}
          <div className="settings-section">
            <p className="section-label">tags and ordering</p>
            <TextField label="tags or raw terms" value={form.tags} onChange={(event) => update("tags", event.target.value)} placeholder="artist_name -sketch" autoFocus fullWidth />
            <div className="field-grid">
              <FormControl>
                <InputLabel id="query-order-label">order</InputLabel>
                <Select labelId="query-order-label" label="order" value={form.order} onChange={(event) => update("order", event.target.value as QueryOrder)}>
                  <MenuItem value="">site default</MenuItem><MenuItem value="score">highest score</MenuItem><MenuItem value="score_asc">lowest score</MenuItem><MenuItem value="id_desc">newest id</MenuItem><MenuItem value="id">oldest id</MenuItem><MenuItem value="mpixels">largest pixels</MenuItem><MenuItem value="mpixels_asc">smallest pixels</MenuItem><MenuItem value="landscape">landscape</MenuItem><MenuItem value="portrait">portrait</MenuItem><MenuItem value="vote">most votes</MenuItem><MenuItem value="random">random</MenuItem>
                </Select>
              </FormControl>
              <FormControl>
                <InputLabel id="query-rating-label">rating</InputLabel>
                <Select labelId="query-rating-label" label="rating" value={form.rating} onChange={(event) => update("rating", event.target.value as AdvancedQueryForm["rating"])}>
                  <MenuItem value="">policy default</MenuItem><MenuItem value="s">safe</MenuItem><MenuItem value="q">questionable</MenuItem><MenuItem value="e">explicit</MenuItem><MenuItem value="-s">exclude safe</MenuItem><MenuItem value="-q">exclude questionable</MenuItem><MenuItem value="-e">exclude explicit</MenuItem>
                </Select>
              </FormControl>
            </div>
          </div>
          <div className="settings-section">
            <p className="section-label">numeric ranges</p>
            <TextField label="exact score" type="number" value={form.score} onChange={(event) => update("score", event.target.value)} placeholder="e.g. 10" fullWidth />
            <div className="field-grid">
              <TextField label="minimum width" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.widthMin} onChange={(event) => update("widthMin", event.target.value)} />
              <TextField label="maximum width" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.widthMax} onChange={(event) => update("widthMax", event.target.value)} />
              <TextField label="minimum height" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.heightMin} onChange={(event) => update("heightMin", event.target.value)} />
              <TextField label="maximum height" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.heightMax} onChange={(event) => update("heightMax", event.target.value)} />
            </div>
            <div className="field-grid">
              <TextField label="minimum score" type="number" value={form.scoreMin} onChange={(event) => update("scoreMin", event.target.value)} />
              <TextField label="maximum score" type="number" value={form.scoreMax} onChange={(event) => update("scoreMax", event.target.value)} />
              <TextField label="minimum post id" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.idMin} onChange={(event) => update("idMin", event.target.value)} />
              <TextField label="maximum post id" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.idMax} onChange={(event) => update("idMax", event.target.value)} />
              <TextField label="minimum votes" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.voteMin} onChange={(event) => update("voteMin", event.target.value)} />
              <TextField label="maximum votes" type="number" slotProps={{ htmlInput: { min: 0 } }} value={form.voteMax} onChange={(event) => update("voteMax", event.target.value)} />
              <TextField label="minimum megapixels" type="number" slotProps={{ htmlInput: { min: 0, step: 0.1 } }} value={form.mpixelsMin} onChange={(event) => update("mpixelsMin", event.target.value)} />
              <TextField label="maximum megapixels" type="number" slotProps={{ htmlInput: { min: 0, step: 0.1 } }} value={form.mpixelsMax} onChange={(event) => update("mpixelsMax", event.target.value)} />
            </div>
          </div>
          <div className="settings-section">
            <p className="section-label">date and identity</p>
            <div className="field-grid">
              <TextField label="date from" type="date" value={form.dateFrom} slotProps={{ htmlInput: { max: today() }, inputLabel: { shrink: true } }} onChange={(event) => update("dateFrom", event.target.value)} />
              <TextField label="date to" type="date" value={form.dateTo} slotProps={{ htmlInput: { max: today() }, inputLabel: { shrink: true } }} onChange={(event) => update("dateTo", event.target.value)} />
              <TextField label="user / author tag" value={form.user} onChange={(event) => update("user", event.target.value)} placeholder="user name" />
              <TextField label="source" value={form.source} onChange={(event) => update("source", event.target.value)} />
              <TextField label="parent id" inputMode="numeric" value={form.parent} onChange={(event) => update("parent", event.target.value)} />
              <TextField label="pool id" inputMode="numeric" value={form.pool} onChange={(event) => update("pool", event.target.value)} />
            </div>
            <TextField label="md5 checksum" value={form.md5} onChange={(event) => update("md5", event.target.value)} fullWidth />
          </div>
        </DialogContent>
        <DialogActions>
          <Button variant="text" type="button" onClick={onClose}>cancel</Button>
          <Button variant="contained" type="submit">search</Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}

interface SettingsDialogProps {
  config: AppConfig | null;
  themeMode: ThemeMode;
  onThemeChange: (theme: ThemeMode) => void;
  onCancel: () => void;
  onSave: (downloadPath: string, contentPolicy: ContentPolicy, downloadVariant: MediaVariant, network: NetworkPolicy) => Promise<void>;
  onDetectProxy: () => Promise<{ detected: boolean; endpoint?: string | null; source?: string | null }>;
  onClearCache: () => Promise<void>;
  formatError: (reason: unknown) => string;
}

export function SettingsDialog({ config, themeMode, onThemeChange, onCancel, onSave, onDetectProxy, onClearCache, formatError }: SettingsDialogProps) {
  const [downloadPath, setDownloadPath] = useState(config?.download_path ?? "");
  const [contentPolicy, setContentPolicy] = useState<ContentPolicy>(config?.content_policy ?? "SafeOnly");
  const [downloadVariant, setDownloadVariant] = useState<MediaVariant>(config?.download_variant ?? "Full");
  const configuredProxy = config?.network.proxy ?? "Auto";
  const [proxyMode, setProxyMode] = useState<"Auto" | "Direct" | "Manual">(typeof configuredProxy === "string" ? configuredProxy : "Manual");
  const [proxyUrl, setProxyUrl] = useState(typeof configuredProxy === "string" ? "" : configuredProxy.Manual.url);
  const [maxRetries, setMaxRetries] = useState(config?.network.max_retries ?? 2);
  const [retryDelayMs, setRetryDelayMs] = useState(config?.network.retry_delay_ms ?? 500);
  const [maxRetryDelayMs, setMaxRetryDelayMs] = useState(config?.network.max_retry_delay_ms ?? 8000);
  const [proxyDetection, setProxyDetection] = useState<string | null>(null);
  const [detecting, setDetecting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [clearCacheOpen, setClearCacheOpen] = useState(false);
  const [clearingCache, setClearingCache] = useState(false);
  const [clearCacheError, setClearCacheError] = useState<string | null>(null);

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
      const result = await onDetectProxy();
      setProxyDetection(result.detected ? `detected ${result.endpoint ?? "a system proxy"} (${result.source ?? "system"})` : "no proxy detected");
    } catch (reason) {
      setProxyDetection(`detection failed: ${formatError(reason)}`);
    } finally {
      setDetecting(false);
    }
  }

  async function handleClearCache() {
    setClearingCache(true);
    setClearCacheError(null);
    try {
      await onClearCache();
      setClearCacheOpen(false);
    } catch (reason) {
      setClearCacheError(formatError(reason));
    } finally {
      setClearingCache(false);
    }
  }

  return (
    <>
      <Dialog open onClose={onCancel} fullWidth maxWidth="sm">
        <form onSubmit={submit}>
          <DialogTitle>
            <p className="eyebrow">app preferences</p>
            settings
            <IconButton type="button" aria-label="close settings" onClick={onCancel} sx={{ position: "absolute", right: 12, top: 12 }}><Icon name="close" /></IconButton>
          </DialogTitle>
          <DialogContent dividers>
          <div className="settings-section">
            <p className="section-label">appearance</p>
            <FormControl fullWidth>
              <InputLabel id="theme-label">theme</InputLabel>
              <Select labelId="theme-label" label="theme" value={themeMode} onChange={(event) => onThemeChange(event.target.value as ThemeMode)}>
                <MenuItem value="system">follow system</MenuItem><MenuItem value="light">light</MenuItem><MenuItem value="dark">dark</MenuItem>
              </Select>
            </FormControl>
            <p className="helper-text">system follows macOS or Windows appearance; light and dark stay fixed.</p>
          </div>
          <div className="settings-section">
            <p className="section-label">storage & site</p>
            <TextField label="download path" required value={downloadPath} onChange={(event) => setDownloadPath(event.target.value)} fullWidth />
            <FormControl fullWidth>
              <InputLabel id="content-policy-label">content policy</InputLabel>
              <Select labelId="content-policy-label" label="content policy" value={contentPolicy} onChange={(event) => setContentPolicy(event.target.value as ContentPolicy)}>
                <MenuItem value="SafeOnly">safe only (default)</MenuItem><MenuItem value="AllowQuestionable">allow questionable</MenuItem><MenuItem value="AllowExplicit">allow explicit</MenuItem><MenuItem value="ExplicitOnly">explicit only</MenuItem>
              </Select>
            </FormControl>
            <FormControl fullWidth>
              <InputLabel id="download-quality-label">download quality</InputLabel>
              <Select labelId="download-quality-label" label="download quality" value={downloadVariant} onChange={(event) => setDownloadVariant(event.target.value as MediaVariant)}>
                <MenuItem value="Full">best available (full)</MenuItem><MenuItem value="Sample">sample</MenuItem><MenuItem value="Preview">preview</MenuItem>
              </Select>
            </FormControl>
            <p className="helper-text">this controls which ratings appear in feeds and searches.</p>
          </div>
            <div className="settings-section">
              <p className="section-label">local cache</p>
              <p className="helper-text">remove temporary download files and detail-image previews. downloaded files, settings, login session, and history stay untouched.</p>
              <Button variant="outlined" color="warning" type="button" onClick={() => { setClearCacheError(null); setClearCacheOpen(true); }}>clear local cache</Button>
            </div>
            <div className="settings-section">
              <p className="section-label">connection resilience</p>
            <FormControl fullWidth>
              <InputLabel id="proxy-mode-label">proxy</InputLabel>
              <Select labelId="proxy-mode-label" label="proxy" value={proxyMode} onChange={(event) => setProxyMode(event.target.value as "Auto" | "Direct" | "Manual")}>
                <MenuItem value="Auto">use system / environment</MenuItem><MenuItem value="Direct">direct connection</MenuItem><MenuItem value="Manual">manual proxy</MenuItem>
              </Select>
            </FormControl>
            {proxyMode === "Manual" && <TextField label="proxy url" required placeholder="http://127.0.0.1:7890" value={proxyUrl} onChange={(event) => setProxyUrl(event.target.value)} fullWidth />}
            <Button variant="outlined" type="button" disabled={detecting} onClick={() => void handleDetectProxy()}>{detecting ? "detecting…" : "detect proxy"}</Button>
            {proxyDetection && <p className="helper-text" role="status">{proxyDetection}</p>}
            <div className="field-grid">
              <TextField label="max retries" type="number" slotProps={{ htmlInput: { min: 0, max: 8 } }} value={maxRetries} onChange={(event) => setMaxRetries(Number(event.target.value))} />
              <TextField label="initial delay" type="number" slotProps={{ htmlInput: { min: 100 } }} value={retryDelayMs} onChange={(event) => setRetryDelayMs(Number(event.target.value))} />
            </div>
            <TextField label="maximum retry delay" type="number" slotProps={{ htmlInput: { min: 100 } }} value={maxRetryDelayMs} onChange={(event) => setMaxRetryDelayMs(Number(event.target.value))} fullWidth />
            <p className="helper-text">429 and temporary server responses retry with a bounded delay.</p>
            </div>
          </DialogContent>
          <DialogActions>
            <Button variant="text" type="button" onClick={onCancel}>cancel</Button>
            <Button variant="contained" type="submit" disabled={saving}>{saving ? "saving…" : "save settings"}</Button>
          </DialogActions>
        </form>
      </Dialog>
      <Dialog open={clearCacheOpen} onClose={() => !clearingCache && setClearCacheOpen(false)} fullWidth maxWidth="xs">
        <DialogTitle>clear local cache?</DialogTitle>
        <DialogContent>
          <p className="helper-text">this removes temporary download staging and cached detail images. it does not remove your configured download library, settings, login session, or download history.</p>
          {clearCacheError && <Alert severity="error">couldn’t clear local cache: {clearCacheError}</Alert>}
        </DialogContent>
        <DialogActions>
          <Button variant="text" type="button" disabled={clearingCache} onClick={() => setClearCacheOpen(false)}>cancel</Button>
          <Button variant="contained" color="warning" type="button" disabled={clearingCache} onClick={() => void handleClearCache()}>{clearingCache ? "clearing…" : "clear cache"}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}

interface ErrorStateProps {
  message: string;
  onRetry: () => void;
  onOpenSite?: () => void;
  siteName?: string;
}

export function ErrorState({ message, onRetry, onOpenSite, siteName }: ErrorStateProps) {
  const detail = message
    .replace("Failed to load images: ", "")
    .replace(/query session has no next page/i, "this feed window has ended; start it again to refresh the results");
  return (
    <Alert
      className="error-state"
      severity="error"
      icon={<span className="error-symbol" aria-hidden="true">!</span>}
      action={<><Button color="inherit" size="small" onClick={onRetry}>try again</Button>{onOpenSite && <Button color="inherit" size="small" onClick={onOpenSite}>open {siteName}</Button>}</>}
    >
      <div><h3>couldn’t load this feed</h3><p>{detail}</p></div>
    </Alert>
  );
}

export function Toast({ state, onClose }: { state: ToastState; onClose: () => void }) {
  return (
    <Snackbar className="toast" open onClose={onClose} autoHideDuration={5000} anchorOrigin={{ vertical: "top", horizontal: "right" }}>
      <Alert onClose={onClose} severity={state.tone} variant="filled" sx={{ width: "100%" }}>
        <strong>{state.title}</strong><div>{state.message}</div>
      </Alert>
    </Snackbar>
  );
}

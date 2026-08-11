use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use dreamland_core::{MediaVariant, NetworkPolicy, Post, SiteId};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::{download_image, DownloadOutcome};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    ExistingTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadRecord {
    pub id: String,
    pub site: SiteId,
    pub post_id: String,
    pub variant: MediaVariant,
    pub status: DownloadStatus,
    pub target_path: Option<String>,
    pub error: Option<String>,
    pub attempts: u32,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub metadata: Post,
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub site: SiteId,
    pub post_id: String,
    pub variant: MediaVariant,
    pub source_url: String,
    pub download_root: PathBuf,
    pub metadata: Post,
}

#[derive(Clone, Default)]
pub struct DownloadCancellation {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl DownloadCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub async fn sleep_or_cancel(&self, duration: std::time::Duration) -> bool {
        if self.is_cancelled() {
            return true;
        }
        tokio::select! {
            _ = self.notify.notified() => true,
            _ = tokio::time::sleep(duration) => false,
        }
    }
}

#[derive(Clone)]
pub struct LocalStateStore {
    path: Arc<PathBuf>,
}

impl LocalStateStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            path: Arc::new(path),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(self.path())?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(connection)
    }

    fn migrate(&self) -> Result<()> {
        let connection = self.connection()?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            connection.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS download_queue (
                    id TEXT PRIMARY KEY NOT NULL,
                    site_id TEXT NOT NULL,
                    post_id TEXT NOT NULL,
                    variant TEXT NOT NULL,
                    source_url TEXT NOT NULL,
                    download_root TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    target_path TEXT,
                    error TEXT,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
                    total_bytes INTEGER,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS download_history (
                    id TEXT PRIMARY KEY NOT NULL,
                    site_id TEXT NOT NULL,
                    post_id TEXT NOT NULL,
                    variant TEXT NOT NULL,
                    source_url TEXT NOT NULL,
                    download_root TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    target_path TEXT,
                    error TEXT,
                    attempts INTEGER NOT NULL,
                    bytes_downloaded INTEGER NOT NULL,
                    total_bytes INTEGER,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_download_queue_order
                    ON download_queue (created_at_ms, id);
                CREATE INDEX IF NOT EXISTS idx_download_history_updated
                    ON download_history (updated_at_ms DESC);
                PRAGMA user_version = 1;
                ",
            )?;
        }
        Ok(())
    }

    pub fn enqueue(&self, request: DownloadRequest) -> Result<DownloadRecord> {
        validate_request(&request)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let metadata_json = serde_json::to_string(&request.metadata)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO download_queue
             (id, site_id, post_id, variant, source_url, download_root, metadata_json,
              status, attempts, bytes_downloaded, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Queued', 0, 0, ?8, ?8)",
            params![
                id,
                request.site.as_str(),
                request.post_id,
                variant_name(request.variant),
                request.source_url,
                request.download_root.to_string_lossy(),
                metadata_json,
                now,
            ],
        )?;
        Ok(DownloadRecord {
            id,
            site: request.site,
            post_id: request.post_id,
            variant: request.variant,
            status: DownloadStatus::Queued,
            target_path: None,
            error: None,
            attempts: 0,
            bytes_downloaded: 0,
            total_bytes: None,
            created_at_ms: now,
            updated_at_ms: now,
            metadata: request.metadata,
        })
    }

    pub(crate) fn claim_next(&self) -> Result<Option<StoredDownload>> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut statement = transaction.prepare(
            "SELECT id, site_id, post_id, variant, source_url, download_root,
                    metadata_json, status, target_path, error, attempts,
                    bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
             FROM download_queue
             WHERE status = 'Queued'
             ORDER BY created_at_ms ASC, id ASC
             LIMIT 1",
        )?;
        let mut job = statement.query_row([], decode_stored_download).optional()?;
        drop(statement);
        if let Some(job) = &mut job {
            let now = now_ms();
            transaction.execute(
                "UPDATE download_queue
                 SET status = 'Running', attempts = attempts + 1, updated_at_ms = ?2
                 WHERE id = ?1",
                params![job.record.id, now],
            )?;
            job.record.status = DownloadStatus::Running;
            job.record.attempts += 1;
            job.record.updated_at_ms = now;
        }
        transaction.commit()?;
        Ok(job)
    }

    pub fn recover_running(&self) -> Result<usize> {
        let connection = self.connection()?;
        let count = connection.execute(
            "UPDATE download_queue
             SET status = 'Queued', error = 'Recovered after application restart', updated_at_ms = ?1
             WHERE status = 'Running'",
            params![now_ms()],
        )?;
        Ok(count)
    }

    pub(crate) fn update_progress(
        &self,
        id: &str,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
    ) -> Result<()> {
        self.connection()?.execute(
            "UPDATE download_queue
             SET bytes_downloaded = ?2, total_bytes = ?3, updated_at_ms = ?4
             WHERE id = ?1",
            params![
                id,
                bytes_downloaded as i64,
                total_bytes.map(|value| value as i64),
                now_ms(),
            ],
        )?;
        Ok(())
    }

    pub fn finish(
        &self,
        id: &str,
        status: DownloadStatus,
        target_path: Option<&Path>,
        error: Option<&str>,
    ) -> Result<DownloadRecord> {
        if !matches!(
            status,
            DownloadStatus::Completed
                | DownloadStatus::Failed
                | DownloadStatus::Cancelled
                | DownloadStatus::ExistingTarget
        ) {
            bail!("queue item cannot finish as {status:?}");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let job = transaction
            .query_row(
                "SELECT id, site_id, post_id, variant, source_url, download_root,
                        metadata_json, status, target_path, error, attempts,
                        bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
                 FROM download_queue WHERE id = ?1",
                params![id],
                decode_stored_download,
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("download queue item not found: {id}"))?;
        if status == DownloadStatus::Cancelled && job.record.status != DownloadStatus::Queued {
            bail!("download queue item is already running");
        }
        let now = now_ms();
        let target_path = target_path.map(|path| path.to_string_lossy().into_owned());
        let error = error.map(str::to_owned);
        transaction.execute(
            "INSERT INTO download_history
             (id, site_id, post_id, variant, source_url, download_root, metadata_json,
              status, target_path, error, attempts, bytes_downloaded, total_bytes,
              created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                job.record.id,
                job.record.site.as_str(),
                job.record.post_id,
                variant_name(job.record.variant),
                job.source_url,
                job.download_root.to_string_lossy(),
                serde_json::to_string(&job.record.metadata)?,
                status_name(status),
                target_path,
                error,
                job.record.attempts,
                job.record.bytes_downloaded as i64,
                job.record.total_bytes.map(|value| value as i64),
                now,
            ],
        )?;
        transaction.execute("DELETE FROM download_queue WHERE id = ?1", params![id])?;
        transaction.commit()?;
        let mut record = job.record;
        record.status = status;
        record.target_path = target_path;
        record.error = error;
        record.updated_at_ms = now;
        Ok(record)
    }

    pub fn cancel_queued(&self, id: &str) -> Result<DownloadRecord> {
        self.finish(
            id,
            DownloadStatus::Cancelled,
            None,
            Some("Cancelled before download"),
        )
    }

    pub fn retry(&self, id: &str) -> Result<DownloadRecord> {
        let connection = self.connection()?;
        let previous = connection
            .query_row(
                "SELECT id, site_id, post_id, variant, source_url, download_root,
                        metadata_json, status, target_path, error, attempts,
                        bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
                 FROM download_history WHERE id = ?1",
                params![id],
                decode_stored_download,
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("download history item not found: {id}"))?;
        if !matches!(
            previous.record.status,
            DownloadStatus::Failed | DownloadStatus::Cancelled
        ) {
            bail!("only failed or cancelled downloads can be retried");
        }
        let request = DownloadRequest {
            site: previous.record.site,
            post_id: previous.record.post_id,
            variant: previous.record.variant,
            source_url: previous.source_url,
            download_root: previous.download_root,
            metadata: previous.record.metadata,
        };
        self.enqueue(request)
    }

    pub fn active(&self, limit: u32) -> Result<Vec<DownloadRecord>> {
        self.list_from(
            "SELECT id, site_id, post_id, variant, source_url, download_root,
                    metadata_json, status, target_path, error, attempts,
                    bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
             FROM download_queue ORDER BY created_at_ms ASC, id ASC LIMIT ?1",
            limit,
        )
    }

    pub fn history(&self, limit: u32) -> Result<Vec<DownloadRecord>> {
        self.list_from(
            "SELECT id, site_id, post_id, variant, source_url, download_root,
                    metadata_json, status, target_path, error, attempts,
                    bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
             FROM download_history ORDER BY updated_at_ms DESC, id DESC LIMIT ?1",
            limit,
        )
    }

    fn list_from(&self, query: &str, limit: u32) -> Result<Vec<DownloadRecord>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(query)?;
        let rows = statement.query_map(params![limit], |row| {
            decode_stored_download(row).map(|job| job.record)
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("decode local download records")
            .map_err(Into::into)
    }
}

#[derive(Debug)]
pub(crate) struct StoredDownload {
    pub record: DownloadRecord,
    pub source_url: String,
    pub download_root: PathBuf,
}

#[derive(Clone)]
pub struct DownloadManager {
    store: LocalStateStore,
    cache_root: Arc<PathBuf>,
    network: Arc<RwLock<NetworkPolicy>>,
    active: Arc<Mutex<HashMap<String, DownloadCancellation>>>,
    notify: Arc<Notify>,
}

impl DownloadManager {
    pub fn open(
        state_path: impl Into<PathBuf>,
        cache_root: impl Into<PathBuf>,
        network: NetworkPolicy,
    ) -> Result<Self> {
        let store = LocalStateStore::open(state_path)?;
        store.recover_running()?;
        Ok(Self {
            store,
            cache_root: Arc::new(cache_root.into()),
            network: Arc::new(RwLock::new(network)),
            active: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        })
    }

    pub fn spawn_worker(&self) {
        let manager = self.clone();
        tokio::spawn(async move { manager.run_worker().await });
    }

    pub fn set_network_policy(&self, network: NetworkPolicy) {
        *self
            .network
            .write()
            .expect("download network lock poisoned") = network;
    }

    pub async fn enqueue(&self, request: DownloadRequest) -> Result<DownloadRecord> {
        let store = self.store.clone();
        let record = tokio::task::spawn_blocking(move || store.enqueue(request)).await??;
        self.notify.notify_one();
        Ok(record)
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        if let Some(cancellation) = self
            .active
            .lock()
            .expect("download active lock poisoned")
            .get(id)
            .cloned()
        {
            cancellation.cancel();
            return Ok(());
        }
        let store = self.store.clone();
        let id = id.to_owned();
        match tokio::task::spawn_blocking({
            let id = id.clone();
            move || store.cancel_queued(&id)
        })
        .await?
        {
            Ok(_) => Ok(()),
            Err(error) => {
                if let Some(cancellation) = self
                    .active
                    .lock()
                    .expect("download active lock poisoned")
                    .get(&id)
                    .cloned()
                {
                    cancellation.cancel();
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    pub async fn retry(&self, id: &str) -> Result<DownloadRecord> {
        let store = self.store.clone();
        let id = id.to_owned();
        let record = tokio::task::spawn_blocking(move || store.retry(&id)).await??;
        self.notify.notify_one();
        Ok(record)
    }

    pub async fn records(&self, limit: u32) -> Result<Vec<DownloadRecord>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let mut active = store.active(limit)?;
            let remaining = limit.saturating_sub(active.len() as u32);
            active.extend(store.history(remaining)?);
            Ok(active)
        })
        .await?
    }

    async fn run_worker(self) {
        loop {
            let store = self.store.clone();
            let job = match tokio::task::spawn_blocking(move || store.claim_next()).await {
                Ok(Ok(job)) => job,
                Ok(Err(error)) => {
                    eprintln!("download queue claim failed: {error:#}");
                    None
                }
                Err(error) => {
                    eprintln!("download queue worker join failed: {error}");
                    None
                }
            };
            let Some(job) = job else {
                self.notify.notified().await;
                continue;
            };

            let cancellation = DownloadCancellation::default();
            self.active
                .lock()
                .expect("download active lock poisoned")
                .insert(job.record.id.clone(), cancellation.clone());
            let network = self
                .network
                .read()
                .expect("download network lock poisoned")
                .clone();
            let cache_root = self.cache_root.clone();
            let result = download_image(
                &job.source_url,
                job.record.site.as_str(),
                &job.record.post_id,
                &job.download_root,
                &cache_root,
                &network,
                &cancellation,
            )
            .await;
            if let Ok(DownloadOutcome::Completed(path)) = &result {
                if let Ok(bytes) = tokio::fs::metadata(path)
                    .await
                    .map(|metadata| metadata.len())
                {
                    let store = self.store.clone();
                    let id = job.record.id.clone();
                    if let Err(error) = tokio::task::spawn_blocking(move || {
                        store.update_progress(&id, bytes, Some(bytes))
                    })
                    .await
                    {
                        eprintln!("download progress update failed: {error}");
                    }
                }
            }
            self.active
                .lock()
                .expect("download active lock poisoned")
                .remove(&job.record.id);

            let (status, target, error) = match result {
                Ok(DownloadOutcome::Completed(path)) => {
                    (DownloadStatus::Completed, Some(path), None)
                }
                Ok(DownloadOutcome::ExistingTarget(path)) => (
                    DownloadStatus::ExistingTarget,
                    Some(path),
                    Some("Target already exists".to_owned()),
                ),
                Ok(DownloadOutcome::Cancelled) => (
                    DownloadStatus::Cancelled,
                    None,
                    Some("Cancelled during download".to_owned()),
                ),
                Err(error) => (DownloadStatus::Failed, None, Some(error.to_string())),
            };
            let store = self.store.clone();
            let id = job.record.id.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || {
                store.finish(&id, status, target.as_deref(), error.as_deref())
            })
            .await
            {
                eprintln!("download queue finish failed: {error}");
            }
        }
    }
}

fn validate_request(request: &DownloadRequest) -> Result<()> {
    let url = reqwest::Url::parse(&request.source_url).context("invalid media URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("media URL must use http or https");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("media URL must not contain credentials");
    }
    validate_component(request.site.as_str(), "site")?;
    if request.post_id.is_empty()
        || !request
            .post_id
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        bail!("post id must be a hexadecimal identifier");
    }
    Ok(())
}

fn validate_component(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        bail!("invalid {label} path component");
    }
    Ok(())
}

fn decode_stored_download(row: &Row<'_>) -> rusqlite::Result<StoredDownload> {
    let site = SiteId::new(row.get::<_, String>(1)?);
    let metadata_json: String = row.get(6)?;
    let metadata = serde_json::from_str(&metadata_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error.to_string(),
            )),
        )
    })?;
    let record = DownloadRecord {
        id: row.get(0)?,
        site,
        post_id: row.get(2)?,
        variant: parse_variant(row.get::<_, String>(3)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error.to_string(),
                )),
            )
        })?,
        status: parse_status(row.get::<_, String>(7)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error.to_string(),
                )),
            )
        })?,
        target_path: row.get(8)?,
        error: row.get(9)?,
        attempts: row.get::<_, i64>(10)? as u32,
        bytes_downloaded: row.get::<_, i64>(11)? as u64,
        total_bytes: row.get::<_, Option<i64>>(12)?.map(|value| value as u64),
        created_at_ms: row.get(13)?,
        updated_at_ms: row.get(14)?,
        metadata,
    };
    Ok(StoredDownload {
        record,
        source_url: row.get(4)?,
        download_root: PathBuf::from(row.get::<_, String>(5)?),
    })
}

fn variant_name(variant: MediaVariant) -> &'static str {
    match variant {
        MediaVariant::Preview => "Preview",
        MediaVariant::Sample => "Sample",
        MediaVariant::Full => "Full",
    }
}

fn parse_variant(value: String) -> Result<MediaVariant> {
    match value.as_str() {
        "Preview" => Ok(MediaVariant::Preview),
        "Sample" => Ok(MediaVariant::Sample),
        "Full" => Ok(MediaVariant::Full),
        _ => bail!("unknown media variant: {value}"),
    }
}

fn status_name(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Queued => "Queued",
        DownloadStatus::Running => "Running",
        DownloadStatus::Completed => "Completed",
        DownloadStatus::Failed => "Failed",
        DownloadStatus::Cancelled => "Cancelled",
        DownloadStatus::ExistingTarget => "ExistingTarget",
    }
}

fn parse_status(value: String) -> Result<DownloadStatus> {
    match value.as_str() {
        "Queued" => Ok(DownloadStatus::Queued),
        "Running" => Ok(DownloadStatus::Running),
        "Completed" => Ok(DownloadStatus::Completed),
        "Failed" => Ok(DownloadStatus::Failed),
        "Cancelled" => Ok(DownloadStatus::Cancelled),
        "ExistingTarget" => Ok(DownloadStatus::ExistingTarget),
        _ => bail!("unknown download status: {value}"),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use dreamland_core::{PostRef, Rating};

    fn test_request(root: &Path) -> DownloadRequest {
        DownloadRequest {
            site: SiteId::new("yandere"),
            post_id: "123".to_owned(),
            variant: MediaVariant::Full,
            source_url: "https://yande.re/image.jpg".to_owned(),
            download_root: root.to_owned(),
            metadata: Post {
                post: PostRef {
                    site: SiteId::new("yandere"),
                    id: "123".to_owned(),
                },
                tags: vec!["artist_name".to_owned()],
                width: Some(100),
                height: Some(100),
                rating: Rating::Safe,
                score: Some(1),
                preview_url: None,
                sample_url: None,
                full_url: Some("https://yande.re/image.jpg".to_owned()),
                file_size: None,
            },
        }
    }

    #[test]
    fn migrations_create_queue_and_history() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = LocalStateStore::open(&path).unwrap();
        let request = test_request(&std::env::temp_dir());
        let record = store.enqueue(request).unwrap();
        assert_eq!(record.status, DownloadStatus::Queued);
        assert_eq!(store.active(10).unwrap().len(), 1);
        assert!(store.history(10).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn running_items_are_requeued_after_restart() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = LocalStateStore::open(&path).unwrap();
        store.enqueue(test_request(&std::env::temp_dir())).unwrap();
        let job = store.claim_next().unwrap().unwrap();
        assert_eq!(job.record.status, DownloadStatus::Running);
        assert_eq!(store.recover_running().unwrap(), 1);
        assert_eq!(store.active(10).unwrap()[0].status, DownloadStatus::Queued);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn queued_cancel_is_terminal_history_without_overwrite() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = LocalStateStore::open(&path).unwrap();
        let record = store.enqueue(test_request(&std::env::temp_dir())).unwrap();
        let cancelled = store.cancel_queued(&record.id).unwrap();
        assert_eq!(cancelled.status, DownloadStatus::Cancelled);
        assert!(store.active(10).unwrap().is_empty());
        assert_eq!(
            store.history(10).unwrap()[0].status,
            DownloadStatus::Cancelled
        );
        std::fs::remove_file(path).unwrap();
    }
}

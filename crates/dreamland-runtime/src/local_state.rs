use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use dreamland_core::{MediaVariant, NetworkPolicy, Post, ReplayableQuery, SavedQuery, SiteId};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::{download_archive, download_image_with_detail_cache, DownloadOutcome};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchiveRecord {
    pub id: String,
    pub site: SiteId,
    pub pool_id: String,
    pub pool_name: String,
    pub status: DownloadStatus,
    pub target_path: Option<String>,
    pub error: Option<String>,
    pub attempts: u32,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct ArchiveRequest {
    pub site: SiteId,
    pub pool_id: String,
    pub pool_name: String,
    pub source_url: String,
    pub download_root: PathBuf,
    pub cookie_header: String,
}

#[derive(Debug)]
pub(crate) struct StoredArchive {
    pub record: ArchiveRecord,
    pub source_url: String,
    pub download_root: PathBuf,
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
        if version < 2 {
            connection.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS saved_queries (
                    id TEXT PRIMARY KEY NOT NULL,
                    site_id TEXT NOT NULL,
                    name TEXT NOT NULL,
                    query_json TEXT NOT NULL,
                    pinned INTEGER NOT NULL DEFAULT 0,
                    position INTEGER NOT NULL DEFAULT 0,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_saved_queries_order
                    ON saved_queries (pinned DESC, position ASC, updated_at_ms DESC);
                PRAGMA user_version = 2;
                ",
            )?;
        }
        if version < 3 {
            connection.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS archive_queue (
                    id TEXT PRIMARY KEY NOT NULL,
                    site_id TEXT NOT NULL,
                    pool_id TEXT NOT NULL,
                    pool_name TEXT NOT NULL,
                    source_url TEXT NOT NULL,
                    download_root TEXT NOT NULL,
                    status TEXT NOT NULL,
                    target_path TEXT,
                    error TEXT,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
                    total_bytes INTEGER,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS archive_history (
                    id TEXT PRIMARY KEY NOT NULL,
                    site_id TEXT NOT NULL,
                    pool_id TEXT NOT NULL,
                    pool_name TEXT NOT NULL,
                    source_url TEXT NOT NULL,
                    download_root TEXT NOT NULL,
                    status TEXT NOT NULL,
                    target_path TEXT,
                    error TEXT,
                    attempts INTEGER NOT NULL,
                    bytes_downloaded INTEGER NOT NULL,
                    total_bytes INTEGER,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_archive_queue_order
                    ON archive_queue (created_at_ms, id);
                CREATE INDEX IF NOT EXISTS idx_archive_history_updated
                    ON archive_history (updated_at_ms DESC);
                PRAGMA user_version = 3;
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
             FROM download_queue ORDER BY created_at_ms ASC, id ASC LIMIT ?1 OFFSET ?2",
            limit,
            0,
        )
    }

    pub fn history(&self, limit: u32) -> Result<Vec<DownloadRecord>> {
        self.history_page(limit, 0)
    }

    pub fn history_page(&self, limit: u32, offset: u32) -> Result<Vec<DownloadRecord>> {
        self.list_from(
            "SELECT id, site_id, post_id, variant, source_url, download_root,
                    metadata_json, status, target_path, error, attempts,
                    bytes_downloaded, total_bytes, created_at_ms, updated_at_ms
             FROM download_history
             ORDER BY updated_at_ms DESC, id DESC LIMIT ?1 OFFSET ?2",
            limit,
            offset,
        )
    }

    pub fn saved_queries(&self, site: &SiteId) -> Result<Vec<SavedQuery>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, site_id, name, query_json, pinned, position, updated_at_ms
             FROM saved_queries
             WHERE site_id = ?1
             ORDER BY pinned DESC, position ASC, updated_at_ms DESC, id ASC",
        )?;
        let rows = statement.query_map(params![site.as_str()], decode_saved_query)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("decode saved queries")
            .map_err(Into::into)
    }

    pub fn save_saved_query(&self, site: &SiteId, mut saved: SavedQuery) -> Result<SavedQuery> {
        if saved.site != *site {
            bail!("saved query site does not match the active site");
        }
        let name = saved.name.trim();
        if name.is_empty() {
            bail!("saved query name cannot be empty");
        }
        saved.name = name.to_owned();
        if saved.id.is_empty() {
            saved.id = uuid::Uuid::new_v4().to_string();
        }
        let now = now_ms();
        saved.updated_at_ms = now;
        let query_json = serde_json::to_string(&saved.query)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO saved_queries
                (id, site_id, name, query_json, pinned, position, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET
                site_id = excluded.site_id,
                name = excluded.name,
                query_json = excluded.query_json,
                pinned = excluded.pinned,
                position = excluded.position,
                updated_at_ms = excluded.updated_at_ms
             WHERE saved_queries.site_id = excluded.site_id",
            params![
                saved.id,
                saved.site.as_str(),
                saved.name,
                query_json,
                saved.pinned,
                saved.position,
                now,
            ],
        )?;
        if connection.changes() != 1 {
            bail!("saved query id belongs to another site");
        }
        Ok(saved)
    }

    pub fn delete_saved_query(&self, site: &SiteId, id: &str) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "DELETE FROM saved_queries WHERE id = ?1 AND site_id = ?2",
            params![id, site.as_str()],
        )?;
        Ok(())
    }

    pub fn move_saved_query(&self, site: &SiteId, id: &str, direction: i8) -> Result<()> {
        if !matches!(direction, -1 | 1) {
            bail!("saved query direction must be -1 or 1");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let pinned: bool = transaction
            .query_row(
                "SELECT pinned FROM saved_queries WHERE id = ?1 AND site_id = ?2",
                params![id, site.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("saved query not found: {id}"))?;
        let mut statement = transaction.prepare(
            "SELECT id FROM saved_queries
             WHERE site_id = ?1 AND pinned = ?2
             ORDER BY position ASC, updated_at_ms ASC, id ASC",
        )?;
        let ids = statement
            .query_map(params![site.as_str(), pinned], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        let current = ids
            .iter()
            .position(|candidate| candidate == id)
            .ok_or_else(|| anyhow::anyhow!("saved query not found: {id}"))?;
        let target = if direction < 0 {
            current.checked_sub(1)
        } else {
            current.checked_add(1).filter(|index| *index < ids.len())
        };
        let Some(target) = target else {
            transaction.commit()?;
            return Ok(());
        };
        let mut reordered = ids;
        reordered.swap(current, target);
        for (position, query_id) in reordered.iter().enumerate() {
            transaction.execute(
                "UPDATE saved_queries SET position = ?1 WHERE id = ?2 AND site_id = ?3",
                params![position as u32, query_id, site.as_str()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn enqueue_archive(&self, request: ArchiveRequest) -> Result<ArchiveRecord> {
        validate_archive_request(&request)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO archive_queue
             (id, site_id, pool_id, pool_name, source_url, download_root, status,
              attempts, bytes_downloaded, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Queued', 0, 0, ?7, ?7)",
            params![
                id,
                request.site.as_str(),
                request.pool_id,
                request.pool_name,
                request.source_url,
                request.download_root.to_string_lossy(),
                now,
            ],
        )?;
        Ok(ArchiveRecord {
            id,
            site: request.site,
            pool_id: request.pool_id,
            pool_name: request.pool_name,
            status: DownloadStatus::Queued,
            target_path: None,
            error: None,
            attempts: 0,
            bytes_downloaded: 0,
            total_bytes: None,
            created_at_ms: now,
            updated_at_ms: now,
        })
    }

    pub(crate) fn claim_next_archive(&self) -> Result<Option<StoredArchive>> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut statement = transaction.prepare(
            "SELECT id, site_id, pool_id, pool_name, source_url, download_root,
                    status, target_path, error, attempts, bytes_downloaded, total_bytes,
                    created_at_ms, updated_at_ms
             FROM archive_queue WHERE status = 'Queued'
             ORDER BY created_at_ms ASC, id ASC LIMIT 1",
        )?;
        let mut archive = statement.query_row([], decode_stored_archive).optional()?;
        drop(statement);
        if let Some(archive) = &mut archive {
            let now = now_ms();
            transaction.execute(
                "UPDATE archive_queue SET status = 'Running', attempts = attempts + 1,
                 updated_at_ms = ?2 WHERE id = ?1",
                params![archive.record.id, now],
            )?;
            archive.record.status = DownloadStatus::Running;
            archive.record.attempts += 1;
            archive.record.updated_at_ms = now;
        }
        transaction.commit()?;
        Ok(archive)
    }

    pub fn archive_records(&self, limit: u32) -> Result<Vec<ArchiveRecord>> {
        let connection = self.connection()?;
        let mut records = Vec::new();
        let mut queue = connection.prepare(
            "SELECT id, site_id, pool_id, pool_name, source_url, download_root,
                    status, target_path, error, attempts, bytes_downloaded, total_bytes,
                    created_at_ms, updated_at_ms
             FROM archive_queue ORDER BY created_at_ms ASC, id ASC LIMIT ?1",
        )?;
        records.extend(
            queue
                .query_map(params![limit], decode_stored_archive)
                .context("decode archive queue")?
                .map(|row| row.map(|archive| archive.record))
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
        let remaining = limit.saturating_sub(records.len() as u32);
        if remaining > 0 {
            let mut history = connection.prepare(
                "SELECT id, site_id, pool_id, pool_name, source_url, download_root,
                        status, target_path, error, attempts, bytes_downloaded, total_bytes,
                        created_at_ms, updated_at_ms
                 FROM archive_history ORDER BY updated_at_ms DESC, id DESC LIMIT ?1",
            )?;
            records.extend(
                history
                    .query_map(params![remaining], decode_stored_archive)
                    .context("decode archive history")?
                    .map(|row| row.map(|archive| archive.record))
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            );
        }
        Ok(records)
    }

    pub fn cancel_queued_archive(&self, id: &str) -> Result<ArchiveRecord> {
        self.finish_archive(
            id,
            DownloadStatus::Cancelled,
            None,
            Some("Cancelled before download"),
        )
    }

    pub fn finish_archive(
        &self,
        id: &str,
        status: DownloadStatus,
        target_path: Option<&Path>,
        error: Option<&str>,
    ) -> Result<ArchiveRecord> {
        if !matches!(
            status,
            DownloadStatus::Completed
                | DownloadStatus::Failed
                | DownloadStatus::Cancelled
                | DownloadStatus::ExistingTarget
        ) {
            bail!("archive item cannot finish as {status:?}");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let archive = transaction
            .query_row(
                "SELECT id, site_id, pool_id, pool_name, source_url, download_root,
                        status, target_path, error, attempts, bytes_downloaded, total_bytes,
                        created_at_ms, updated_at_ms
                 FROM archive_queue WHERE id = ?1",
                params![id],
                decode_stored_archive,
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("archive queue item not found: {id}"))?;
        let now = now_ms();
        let target_path = target_path.map(|path| path.to_string_lossy().into_owned());
        let error = error.map(str::to_owned);
        transaction.execute(
            "INSERT INTO archive_history
             (id, site_id, pool_id, pool_name, source_url, download_root, status,
              target_path, error, attempts, bytes_downloaded, total_bytes,
              created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                archive.record.id,
                archive.record.site.as_str(),
                archive.record.pool_id,
                archive.record.pool_name,
                archive.source_url,
                archive.download_root.to_string_lossy(),
                status_name(status),
                target_path,
                error,
                archive.record.attempts,
                archive.record.bytes_downloaded as i64,
                archive.record.total_bytes.map(|value| value as i64),
                archive.record.created_at_ms,
                now,
            ],
        )?;
        transaction.execute("DELETE FROM archive_queue WHERE id = ?1", params![id])?;
        transaction.commit()?;
        let mut record = archive.record;
        record.status = status;
        record.target_path = target_path;
        record.error = error;
        record.updated_at_ms = now;
        Ok(record)
    }

    pub fn recover_running_archives(&self) -> Result<usize> {
        let connection = self.connection()?;
        Ok(connection.execute(
            "UPDATE archive_queue SET status = 'Queued', updated_at_ms = ?1 WHERE status = 'Running'",
            params![now_ms()],
        )?)
    }

    fn list_from(&self, query: &str, limit: u32, offset: u32) -> Result<Vec<DownloadRecord>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(query)?;
        let rows = statement.query_map(params![limit, offset], |row| {
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
    detail_cache_root: Arc<RwLock<Option<PathBuf>>>,
    network: Arc<RwLock<NetworkPolicy>>,
    active: Arc<Mutex<HashMap<String, DownloadCancellation>>>,
    archive_cookies: Arc<Mutex<HashMap<String, String>>>,
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
        store.recover_running_archives()?;
        Ok(Self {
            store,
            cache_root: Arc::new(cache_root.into()),
            detail_cache_root: Arc::new(RwLock::new(None)),
            network: Arc::new(RwLock::new(network)),
            active: Arc::new(Mutex::new(HashMap::new())),
            archive_cookies: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        })
    }

    pub fn worker(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let manager = self.clone();
        async move { manager.run_worker().await }
    }

    pub fn archive_worker(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let manager = self.clone();
        async move { manager.run_archive_worker().await }
    }

    pub fn set_network_policy(&self, network: NetworkPolicy) {
        *self
            .network
            .write()
            .expect("download network lock poisoned") = network;
    }

    pub fn set_detail_cache_root(&self, path: impl Into<PathBuf>) {
        *self
            .detail_cache_root
            .write()
            .expect("detail cache lock poisoned") = Some(path.into());
    }

    pub async fn clear_cache(&self) -> Result<()> {
        if !self
            .active
            .lock()
            .expect("download active lock poisoned")
            .is_empty()
        {
            bail!("cannot clear cache while downloads are active");
        }
        let download_cache = self.cache_root.as_ref().clone();
        let detail_cache = self
            .detail_cache_root
            .read()
            .expect("detail cache lock poisoned")
            .clone()
            .unwrap_or_else(crate::default_detail_cache_path);
        let detail_staging = detail_cache.with_file_name("detail-staging");
        tokio::task::spawn_blocking(move || {
            for path in [download_cache, detail_cache, detail_staging] {
                if path.exists() {
                    std::fs::remove_dir_all(path)?;
                }
            }
            Ok::<_, std::io::Error>(())
        })
        .await??;
        Ok(())
    }

    pub async fn enqueue(&self, request: DownloadRequest) -> Result<DownloadRecord> {
        let store = self.store.clone();
        let record = tokio::task::spawn_blocking(move || store.enqueue(request)).await??;
        self.notify.notify_one();
        Ok(record)
    }

    pub async fn enqueue_archive(&self, request: ArchiveRequest) -> Result<ArchiveRecord> {
        let cookie = request.cookie_header.clone();
        let store = self.store.clone();
        let record = tokio::task::spawn_blocking(move || store.enqueue_archive(request)).await??;
        self.archive_cookies
            .lock()
            .expect("archive cookie lock poisoned")
            .insert(record.id.clone(), cookie);
        self.notify.notify_one();
        Ok(record)
    }

    pub async fn archives(&self, limit: u32) -> Result<Vec<ArchiveRecord>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.archive_records(limit)).await?
    }

    pub async fn cancel_archive(&self, id: &str) -> Result<()> {
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
        let store_id = id.clone();
        tokio::task::spawn_blocking(move || store.cancel_queued_archive(&store_id)).await??;
        self.archive_cookies
            .lock()
            .expect("archive cookie lock poisoned")
            .remove(&id);
        Ok(())
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
            active.extend(store.history_page(limit, 0)?);
            Ok(active)
        })
        .await?
    }

    pub async fn history_page(&self, limit: u32, offset: u32) -> Result<Vec<DownloadRecord>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.history_page(limit, offset)).await?
    }

    pub async fn saved_queries(&self, site: &SiteId) -> Result<Vec<SavedQuery>> {
        let store = self.store.clone();
        let site = site.clone();
        tokio::task::spawn_blocking(move || store.saved_queries(&site)).await?
    }

    pub async fn save_saved_query(&self, site: &SiteId, saved: SavedQuery) -> Result<SavedQuery> {
        let store = self.store.clone();
        let site = site.clone();
        tokio::task::spawn_blocking(move || store.save_saved_query(&site, saved)).await?
    }

    pub async fn delete_saved_query(&self, site: &SiteId, id: &str) -> Result<()> {
        let store = self.store.clone();
        let site = site.clone();
        let id = id.to_owned();
        tokio::task::spawn_blocking(move || store.delete_saved_query(&site, &id)).await?
    }

    pub async fn move_saved_query(&self, site: &SiteId, id: &str, direction: i8) -> Result<()> {
        let store = self.store.clone();
        let site = site.clone();
        let id = id.to_owned();
        tokio::task::spawn_blocking(move || store.move_saved_query(&site, &id, direction)).await?
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
            let detail_cache_root = self
                .detail_cache_root
                .read()
                .expect("detail cache lock poisoned")
                .clone();
            let result = download_image_with_detail_cache(
                &job.source_url,
                job.record.site.as_str(),
                &job.record.post_id,
                &job.download_root,
                &cache_root,
                detail_cache_root.as_deref(),
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
                Ok(DownloadOutcome::ExistingTarget(path)) => {
                    (DownloadStatus::ExistingTarget, Some(path), None)
                }
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

    async fn run_archive_worker(self) {
        loop {
            let store = self.store.clone();
            let archive =
                match tokio::task::spawn_blocking(move || store.claim_next_archive()).await {
                    Ok(Ok(archive)) => archive,
                    Ok(Err(error)) => {
                        eprintln!("archive queue claim failed: {error:#}");
                        None
                    }
                    Err(error) => {
                        eprintln!("archive queue worker join failed: {error}");
                        None
                    }
                };
            let Some(archive) = archive else {
                self.notify.notified().await;
                continue;
            };
            let cancellation = DownloadCancellation::default();
            self.active
                .lock()
                .expect("download active lock poisoned")
                .insert(archive.record.id.clone(), cancellation.clone());
            let cookie = self
                .archive_cookies
                .lock()
                .expect("archive cookie lock poisoned")
                .get(&archive.record.id)
                .cloned();
            let network = self
                .network
                .read()
                .expect("download network lock poisoned")
                .clone();
            let result = match cookie {
                Some(cookie) => {
                    download_archive(
                        &archive.source_url,
                        archive.record.site.as_str(),
                        &archive.record.pool_id,
                        &archive.record.pool_name,
                        &archive.download_root,
                        &self.cache_root,
                        &network,
                        &cookie,
                        &cancellation,
                    )
                    .await
                }
                None => Err(anyhow::anyhow!(
                    "yandere login session is unavailable; sign in again and retry"
                )),
            };
            self.active
                .lock()
                .expect("download active lock poisoned")
                .remove(&archive.record.id);
            self.archive_cookies
                .lock()
                .expect("archive cookie lock poisoned")
                .remove(&archive.record.id);
            let (status, target, error) = match result {
                Ok(DownloadOutcome::Completed(path)) => {
                    (DownloadStatus::Completed, Some(path), None)
                }
                Ok(DownloadOutcome::ExistingTarget(path)) => {
                    (DownloadStatus::ExistingTarget, Some(path), None)
                }
                Ok(DownloadOutcome::Cancelled) => (
                    DownloadStatus::Cancelled,
                    None,
                    Some("Cancelled during archive download".to_owned()),
                ),
                Err(error) => (DownloadStatus::Failed, None, Some(error.to_string())),
            };
            let store = self.store.clone();
            let id = archive.record.id.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || {
                store.finish_archive(&id, status, target.as_deref(), error.as_deref())
            })
            .await
            {
                eprintln!("archive queue finish failed: {error}");
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

fn decode_saved_query(row: &Row<'_>) -> rusqlite::Result<SavedQuery> {
    let query_json: String = row.get(3)?;
    let query = serde_json::from_str::<ReplayableQuery>(&query_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(SavedQuery {
        id: row.get(0)?,
        site: SiteId::new(row.get::<_, String>(1)?),
        name: row.get(2)?,
        query,
        pinned: row.get::<_, i64>(4)? != 0,
        position: row.get::<_, i64>(5)?.try_into().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Integer,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "saved query position is negative",
                )),
            )
        })?,
        updated_at_ms: row.get(6)?,
    })
}

fn decode_stored_archive(row: &Row<'_>) -> rusqlite::Result<StoredArchive> {
    let site = SiteId::new(row.get::<_, String>(1)?);
    let record = ArchiveRecord {
        id: row.get(0)?,
        site,
        pool_id: row.get(2)?,
        pool_name: row.get(3)?,
        status: parse_status(row.get::<_, String>(6)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error.to_string(),
                )),
            )
        })?,
        target_path: row.get(7)?,
        error: row.get(8)?,
        attempts: row.get::<_, i64>(9)? as u32,
        bytes_downloaded: row.get::<_, i64>(10)? as u64,
        total_bytes: row.get::<_, Option<i64>>(11)?.map(|value| value as u64),
        created_at_ms: row.get(12)?,
        updated_at_ms: row.get(13)?,
    };
    Ok(StoredArchive {
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

fn validate_archive_request(request: &ArchiveRequest) -> Result<()> {
    let url = reqwest::Url::parse(&request.source_url).context("invalid archive URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("archive URL must use http or https");
    }
    validate_component(request.site.as_str(), "site")?;
    if request.pool_id.is_empty() || !request.pool_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("pool id must be numeric");
    }
    if request.pool_name.trim().is_empty() {
        bail!("pool name cannot be empty");
    }
    Ok(())
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
                author: None,
                creator_id: None,
                md5: None,
                source: None,
                parent_id: None,
                has_children: false,
                created_at: None,
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
    fn saved_queries_round_trip_and_delete() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let yandere = SiteId::new("yandere");
        let store = LocalStateStore::open(&path).unwrap();
        let saved = store
            .save_saved_query(
                &yandere,
                SavedQuery {
                    id: String::new(),
                    site: yandere.clone(),
                    name: "Pinned blue search".to_owned(),
                    query: ReplayableQuery {
                        source: dreamland_core::DiscoverySource::Search {
                            expression: "blue_eyes".to_owned(),
                        },
                        content_policy: dreamland_core::ContentPolicy::SafeOnly,
                    },
                    pinned: true,
                    position: 0,
                    updated_at_ms: 0,
                },
            )
            .unwrap();
        let listed = store.saved_queries(&yandere).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], saved);
        store.delete_saved_query(&yandere, &saved.id).unwrap();
        assert!(store.saved_queries(&yandere).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn saved_queries_survive_reopen_and_remain_site_isolated() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let yandere = SiteId::new("yandere");
        let pixiv = SiteId::new("pixiv");
        let saved = SavedQuery {
            id: String::new(),
            site: yandere.clone(),
            name: "Pinned blue search".to_owned(),
            query: ReplayableQuery {
                source: dreamland_core::DiscoverySource::Search {
                    expression: "blue_eyes".to_owned(),
                },
                content_policy: dreamland_core::ContentPolicy::AllowQuestionable,
            },
            pinned: true,
            position: 7,
            updated_at_ms: 0,
        };
        let saved = {
            let store = LocalStateStore::open(&path).unwrap();
            let saved = store.save_saved_query(&yandere, saved).unwrap();
            store
                .save_saved_query(
                    &pixiv,
                    SavedQuery {
                        id: String::new(),
                        site: pixiv.clone(),
                        name: "Pixiv search".to_owned(),
                        query: saved.query.clone(),
                        pinned: false,
                        position: 0,
                        updated_at_ms: 0,
                    },
                )
                .unwrap();
            saved
        };
        let reopened = LocalStateStore::open(&path).unwrap();
        assert_eq!(
            reopened.saved_queries(&yandere).unwrap(),
            vec![saved.clone()]
        );
        assert_eq!(reopened.saved_queries(&pixiv).unwrap().len(), 1);
        assert!(reopened
            .save_saved_query(
                &pixiv,
                SavedQuery {
                    site: pixiv.clone(),
                    id: saved.id.clone(),
                    ..saved.clone()
                },
            )
            .is_err());
        reopened.delete_saved_query(&yandere, &saved.id).unwrap();
        assert!(reopened.saved_queries(&yandere).unwrap().is_empty());
        assert_eq!(reopened.saved_queries(&pixiv).unwrap().len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn saved_queries_move_within_their_pin_group() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let yandere = SiteId::new("yandere");
        let store = LocalStateStore::open(&path).unwrap();
        let mut ids = Vec::new();
        for (name, pinned, position) in [
            ("one", false, 0),
            ("two", false, 1),
            ("three", false, 2),
            ("pinned", true, 0),
        ] {
            ids.push(
                store
                    .save_saved_query(
                        &yandere,
                        SavedQuery {
                            id: String::new(),
                            site: yandere.clone(),
                            name: name.to_owned(),
                            query: ReplayableQuery {
                                source: dreamland_core::DiscoverySource::Search {
                                    expression: name.to_owned(),
                                },
                                content_policy: dreamland_core::ContentPolicy::SafeOnly,
                            },
                            pinned,
                            position,
                            updated_at_ms: 0,
                        },
                    )
                    .unwrap()
                    .id,
            );
        }
        store.move_saved_query(&yandere, &ids[1], -1).unwrap();
        let listed = store.saved_queries(&yandere).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|saved| saved.name.as_str())
                .collect::<Vec<_>>(),
            ["pinned", "two", "one", "three"]
        );
        store.move_saved_query(&yandere, &ids[1], 1).unwrap();
        let listed = store.saved_queries(&yandere).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|saved| saved.name.as_str())
                .collect::<Vec<_>>(),
            ["pinned", "one", "two", "three"]
        );
        store.move_saved_query(&yandere, &ids[3], -1).unwrap();
        let listed = store.saved_queries(&yandere).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|saved| saved.name.as_str())
                .collect::<Vec<_>>(),
            ["pinned", "one", "two", "three"]
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archive_queue_round_trip_and_restart_recovery() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = LocalStateStore::open(&path).unwrap();
        let request = ArchiveRequest {
            site: SiteId::new("yandere"),
            pool_id: "42".to_owned(),
            pool_name: "Art Book".to_owned(),
            source_url: "https://yande.re/pool/zip/42".to_owned(),
            download_root: std::env::temp_dir(),
            cookie_header: "_session=transient".to_owned(),
        };
        let queued = store.enqueue_archive(request).unwrap();
        assert_eq!(store.archive_records(10).unwrap(), vec![queued.clone()]);
        let claimed = store.claim_next_archive().unwrap().unwrap();
        assert_eq!(claimed.record.status, DownloadStatus::Running);
        assert_eq!(store.recover_running_archives().unwrap(), 1);
        assert_eq!(
            store.archive_records(10).unwrap()[0].status,
            DownloadStatus::Queued
        );
        let claimed = store.claim_next_archive().unwrap().unwrap();
        let finished = store
            .finish_archive(
                &claimed.record.id,
                DownloadStatus::Completed,
                Some(Path::new("/tmp/yandere/pools/pool-42_art-book.zip")),
                None,
            )
            .unwrap();
        assert_eq!(finished.status, DownloadStatus::Completed);
        assert_eq!(store.archive_records(10).unwrap()[0], finished);
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

    #[tokio::test]
    async fn clear_cache_keeps_user_library_and_state() {
        let root = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let state_path = root.join("state.sqlite3");
        let download_cache = root.join("downloads");
        let detail_cache = root.join("detail");
        let detail_staging = root.join("detail-staging");
        let library = root.join("library");
        std::fs::create_dir_all(&download_cache).unwrap();
        std::fs::create_dir_all(&detail_cache).unwrap();
        std::fs::create_dir_all(&detail_staging).unwrap();
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(download_cache.join("staged.part"), b"temporary").unwrap();
        std::fs::write(detail_cache.join("preview.jpg"), b"temporary").unwrap();
        std::fs::write(detail_staging.join("staged.part"), b"temporary").unwrap();
        std::fs::write(library.join("keep.jpg"), b"download").unwrap();

        let manager =
            DownloadManager::open(&state_path, &download_cache, NetworkPolicy::default()).unwrap();
        manager.set_detail_cache_root(&detail_cache);
        manager.clear_cache().await.unwrap();

        assert!(!download_cache.exists());
        assert!(!detail_cache.exists());
        assert!(!detail_staging.exists());
        assert!(library.join("keep.jpg").exists());
        assert!(state_path.exists());
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn history_page_supports_loading_older_records() {
        let path = std::env::temp_dir().join(format!("dreamland-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = LocalStateStore::open(&path).unwrap();
        let first = store.enqueue(test_request(&std::env::temp_dir())).unwrap();
        let second = store.enqueue(test_request(&std::env::temp_dir())).unwrap();
        store.cancel_queued(&first.id).unwrap();
        store.cancel_queued(&second.id).unwrap();
        assert_eq!(store.history_page(1, 0).unwrap().len(), 1);
        assert_eq!(store.history_page(1, 1).unwrap().len(), 1);
        std::fs::remove_file(path).unwrap();
    }
}

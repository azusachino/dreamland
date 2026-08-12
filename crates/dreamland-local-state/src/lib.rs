use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const CURRENT_SCHEMA_VERSION: i64 = 3;

/// SQLite infrastructure owned by the local-state crate.
///
/// Runtime use cases receive connections from this repository boundary; they
/// do not decide the database path, busy timeout, or migration order.
#[derive(Clone)]
pub struct SqliteStateRepository {
    path: Arc<PathBuf>,
}

impl SqliteStateRepository {
    pub fn open(path: impl Into<PathBuf>) -> rusqlite::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let repository = Self {
            path: Arc::new(path),
        };
        let connection = repository.connection()?;
        migrate(&connection)?;
        Ok(repository)
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn connection(&self) -> rusqlite::Result<Connection> {
        let connection = Connection::open(self.path())?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(connection)
    }
}

/// Apply the runtime-owned SQLite schema migrations to an open connection.
/// Queue and repository behavior stays in dreamland-runtime; this crate owns
/// only the durable schema contract and its versioned upgrades.
pub fn migrate(connection: &Connection) -> rusqlite::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_reaches_current_schema_and_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        migrate(&connection).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        let table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('download_queue', 'download_history', 'saved_queries', 'archive_queue', 'archive_history')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 5);
    }
}

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// SQLite infrastructure owned by the state crate.
///
/// Runtime use cases receive connections from this repository boundary; they
/// do not decide the database path, busy timeout, or schema details.
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
        initialize_schema(&connection)?;
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

/// Create the current runtime-owned SQLite schema on an open connection.
/// Queue and repository behavior stays in dreamland-runtime; this crate owns
/// only the current durable schema.
pub fn initialize_schema(connection: &Connection) -> rusqlite::Result<()> {
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
            ",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_initialization_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_schema(&connection).unwrap();
        initialize_schema(&connection).unwrap();
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

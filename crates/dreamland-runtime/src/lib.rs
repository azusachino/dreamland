mod config;
mod local_state;
mod media;
mod sessions;

pub use config::{detect_proxy, AppConfig, ProxyDetection};
pub use local_state::{
    ArchiveRecord, ArchiveRequest, DownloadCancellation, DownloadManager, DownloadRecord,
    DownloadRequest, DownloadStatus, LocalStateStore,
};
pub use media::{
    cache_detail_image, cache_detail_image_at, download_archive, download_image,
    download_image_with_detail_cache, find_cached_detail_image_at, log_detail_failure,
    DownloadOutcome,
};
pub use sessions::{QuerySession, QuerySessionStore, SessionOperation};

pub fn default_state_path() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::data_local_dir)
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("state.sqlite3")
}

pub fn default_cache_path() -> std::path::PathBuf {
    default_cache_root().join("downloads")
}

pub fn default_detail_cache_path() -> std::path::PathBuf {
    default_cache_root().join("detail")
}

fn default_cache_root() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
}

pub fn default_log_path() -> std::path::PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::data_local_dir)
        .or_else(dirs::data_dir)
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("logs")
        .join("dreamland.log")
}

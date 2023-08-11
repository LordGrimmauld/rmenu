//! RMenu Plugin Result Cache
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use once_cell::sync::Lazy;
use rmenu_plugin::Entry;
use thiserror::Error;

use crate::config::{CacheSetting, PluginConfig};
use crate::CONFIG_DIR;

static CONFIG_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from(shellexpand::tilde(CONFIG_DIR).to_string()));

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache Not Available")]
    NotAvailable,
    #[error("Cache Invalid")]
    InvalidCache,
    #[error("Cache Expired")]
    CacheExpired,
    #[error("Cache File Error")]
    FileError(#[from] std::io::Error),
    #[error("Encoding Error")]
    EncodingError(#[from] bincode::Error),
}

#[inline]
fn cache_file(name: &str) -> PathBuf {
    CONFIG_PATH.join(format!("{name}.cache"))
}

/// Read Entries from Cache (if Valid and Available)
pub fn read_cache(name: &str, cfg: &PluginConfig) -> Result<Vec<Entry>, CacheError> {
    // confirm cache exists
    let path = cache_file(name);
    if !path.exists() {
        return Err(CacheError::NotAvailable);
    }
    // get file modified date
    let meta = path.metadata()?;
    let modified = meta.modified()?;
    // confirm cache is not expired
    match cfg.cache {
        CacheSetting::NoCache => return Err(CacheError::InvalidCache),
        CacheSetting::Never => {}
        CacheSetting::OnLogin => {
            if let Ok(record) = lastlog::search_self() {
                if let Some(last) = record.last_login.into() {
                    if modified <= last {
                        return Err(CacheError::CacheExpired);
                    }
                }
            }
        }
        CacheSetting::AfterSeconds(secs) => {
            let now = SystemTime::now();
            let duration = Duration::from_secs(secs as u64);
            let diff = now
                .duration_since(modified)
                .unwrap_or_else(|_| Duration::from_secs(0));
            if diff >= duration {
                return Err(CacheError::CacheExpired);
            }
        }
    }
    // attempt to read content
    let data = fs::read(path)?;
    let results: Vec<Entry> = bincode::deserialize(&data)?;
    Ok(results)
}

/// Write Results to Cache (if Allowed)
pub fn write_cache(name: &str, cfg: &PluginConfig, entries: &Vec<Entry>) -> Result<(), CacheError> {
    // write cache if allowed
    match cfg.cache {
        CacheSetting::NoCache => {}
        _ => {
            let path = cache_file(name);
            let data = bincode::serialize(entries)?;
            let mut f = fs::File::create(path)?;
            f.write_all(&data)?;
        }
    }
    Ok(())
}
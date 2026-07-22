use crate::sanitize_path_component;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub file_count: u64,
    pub byte_count: u64,
}

#[derive(Clone)]
pub struct CacheStore {
    root: Arc<PathBuf>,
}

impl CacheStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(Self {
            root: Arc::new(root),
        })
    }

    pub fn path_for(&self, namespace: &str, key: &str, extension: &str) -> Result<PathBuf, String> {
        let namespace = sanitize_path_component(namespace, "media");
        let key = sanitize_path_component(key, "cache");
        let extension = sanitize_path_component(extension.trim_start_matches('.'), "bin");
        let dir = self.root.join(namespace);
        fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        Ok(dir.join(format!("{key}.{extension}")))
    }

    pub fn stats(&self) -> CacheStats {
        let mut stats = CacheStats {
            file_count: 0,
            byte_count: 0,
        };
        collect_stats(self.root.as_path(), &mut stats);
        stats
    }

    pub fn clear_namespace(&self, namespace: &str) -> Result<(), String> {
        let dir = self.root.join(sanitize_path_component(namespace, "media"));

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
        }

        Ok(())
    }

    pub fn prune_to_bytes(&self, maximum_bytes: u64) -> Result<CacheStats, String> {
        let mut files = Vec::new();
        collect_files(self.root.as_path(), &mut files);
        let mut byte_count = files.iter().map(|file| file.byte_count).sum::<u64>();

        files.sort_by_key(|file| file.modified);

        for file in files {
            if byte_count <= maximum_bytes {
                break;
            }

            if fs::remove_file(&file.path).is_ok() {
                byte_count = byte_count.saturating_sub(file.byte_count);
            }
        }

        Ok(self.stats())
    }
}

struct CacheFile {
    path: PathBuf,
    byte_count: u64,
    modified: SystemTime,
}

fn collect_files(path: &std::path::Path, files: &mut Vec<CacheFile>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_files(&path, files);
        } else if let Ok(metadata) = entry.metadata() {
            files.push(CacheFile {
                path,
                byte_count: metadata.len(),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
}

fn collect_stats(path: &std::path::Path, stats: &mut CacheStats) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_stats(&path, stats);
        } else if let Ok(metadata) = entry.metadata() {
            stats.file_count = stats.file_count.saturating_add(1);
            stats.byte_count = stats.byte_count.saturating_add(metadata.len());
        }
    }
}

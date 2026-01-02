// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Persistent cache system for symbol tables and parsed data

use anyhow::Result;
use dashmap::DashMap;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Cache metadata for validation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheMetadata {
    /// Version of the cache format
    pub version: u32,
    /// Timestamp when cache was created
    pub created_at: SystemTime,
    /// Map of file paths to their modification times at cache time
    pub file_mtimes: HashMap<PathBuf, SystemTime>,
}

/// Persistent cache manager
pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    /// Create a new cache manager for the given workspace root
    pub fn new(workspace_root: &Path) -> Self {
        let cache_dir = workspace_root.join(".cherry");
        Self { cache_dir }
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Ensure cache directory exists
    pub fn ensure_cache_dir(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)?;
            log::info!("Created cache directory: {}", self.cache_dir.display());
        }
        Ok(())
    }

    /// Get metadata file path
    fn metadata_path(&self) -> PathBuf {
        self.cache_dir.join("metadata.json")
    }

    /// Get symbol table cache path
    fn symbol_table_path(&self) -> PathBuf {
        self.cache_dir.join("symbols.bin")
    }

    /// Get interner cache path
    fn interner_path(&self) -> PathBuf {
        self.cache_dir.join("interner.bin")
    }

    /// Get include index cache path
    fn include_index_path(&self) -> PathBuf {
        self.cache_dir.join("includes.bin")
    }

    /// Check if cache is valid for the given files
    pub fn is_cache_valid(&self, indexed_files: &[PathBuf]) -> Result<bool> {
        // Check if metadata file exists
        let metadata_path = self.metadata_path();
        if !metadata_path.exists() {
            log::info!("Cache metadata not found");
            return Ok(false);
        }

        // Load metadata
        let metadata_content = fs::read_to_string(&metadata_path)?;
        let metadata: CacheMetadata = serde_json::from_str(&metadata_content)?;

        // Check version
        const CURRENT_VERSION: u32 = 1;
        if metadata.version != CURRENT_VERSION {
            log::info!("Cache version mismatch: {} vs {}", metadata.version, CURRENT_VERSION);
            return Ok(false);
        }

        // Check if all files are still the same
        for file_path in indexed_files {
            if let Some(&cached_mtime) = metadata.file_mtimes.get(file_path) {
                // Check current mtime
                if let Ok(current_metadata) = fs::metadata(file_path) {
                    if let Ok(current_mtime) = current_metadata.modified() {
                        if current_mtime != cached_mtime {
                            log::info!("File modified: {}", file_path.display());
                            return Ok(false);
                        }
                    }
                }
            } else {
                // File not in cache
                log::info!("File not in cache: {}", file_path.display());
                return Ok(false);
            }
        }

        log::info!("Cache is valid");
        Ok(true)
    }

    /// Save cache metadata
    pub fn save_metadata(&self, indexed_files: &[PathBuf]) -> Result<()> {
        self.ensure_cache_dir()?;

        // Collect file modification times
        let mut file_mtimes = HashMap::new();
        for file_path in indexed_files {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(mtime) = metadata.modified() {
                    file_mtimes.insert(file_path.clone(), mtime);
                }
            }
        }

        let metadata = CacheMetadata {
            version: 1,
            created_at: SystemTime::now(),
            file_mtimes,
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(self.metadata_path(), metadata_json)?;

        log::info!("Saved cache metadata for {} files", indexed_files.len());
        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            log::info!("Cleared cache directory");
        }
        Ok(())
    }

    /// Build include index from file paths
    /// Maps filename -> full path for instant lookup
    pub fn build_include_index(files: &[PathBuf]) -> HashMap<String, PathBuf> {
        let mut index = HashMap::new();

        for path in files {
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    // Store by filename for quick lookup
                    index.insert(name_str.to_string(), path.clone());
                }
            }
        }

        log::info!("Built include index with {} entries", index.len());
        index
    }

    /// Save include index to disk using bincode for speed
    pub fn save_include_index(&self, index: &HashMap<String, PathBuf>) -> Result<()> {
        self.ensure_cache_dir()?;

        let data = bincode::serialize(index)?;
        fs::write(self.include_index_path(), data)?;

        log::info!("Saved include index with {} entries to {}",
            index.len(), self.include_index_path().display());
        Ok(())
    }

    /// Load include index from disk
    pub fn load_include_index(&self) -> Result<HashMap<String, PathBuf>> {
        let path = self.include_index_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let data = fs::read(&path)?;
        let index: HashMap<String, PathBuf> = bincode::deserialize(&data)?;

        log::info!("Loaded include index with {} entries from {}",
            index.len(), path.display());
        Ok(index)
    }

    /// Populate DashMap from HashMap for runtime use
    pub fn populate_dashmap(dashmap: &DashMap<String, PathBuf>, hashmap: HashMap<String, PathBuf>) {
        for (key, value) in hashmap {
            dashmap.insert(key, value);
        }
    }
}

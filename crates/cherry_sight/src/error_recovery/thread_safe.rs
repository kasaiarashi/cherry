// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Thread-safe analysis coordination

use crate::util::FileId;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::{HashMap, HashSet};

/// Analysis lock for coordinating concurrent access
#[derive(Debug, Clone)]
pub struct AnalysisLock {
    locked_files: Arc<RwLock<HashSet<FileId>>>,
}

impl AnalysisLock {
    pub fn new() -> Self {
        Self {
            locked_files: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Try to acquire lock for file analysis
    pub fn try_lock(&self, file_id: FileId) -> bool {
        let mut locked = self.locked_files.write().unwrap();
        if locked.contains(&file_id) {
            false
        } else {
            locked.insert(file_id);
            true
        }
    }

    /// Release lock for file
    pub fn unlock(&self, file_id: FileId) {
        self.locked_files.write().unwrap().remove(&file_id);
    }

    /// Check if file is locked
    pub fn is_locked(&self, file_id: FileId) -> bool {
        self.locked_files.read().unwrap().contains(&file_id)
    }

    /// Get count of locked files
    pub fn locked_count(&self) -> usize {
        self.locked_files.read().unwrap().len()
    }
}

impl Default for AnalysisLock {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe analysis coordinator
pub struct ThreadSafeAnalyzer {
    locks: AnalysisLock,
    in_progress: Arc<Mutex<HashMap<FileId, AnalysisProgress>>>,
}

/// Analysis progress for a file
#[derive(Debug, Clone)]
pub struct AnalysisProgress {
    pub file_id: FileId,
    pub stage: AnalysisStage,
    pub progress_percent: u8,
}

/// Stage of analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStage {
    /// Parsing source file
    Parsing,
    /// Building symbol table
    Indexing,
    /// Performing semantic analysis
    Semantic,
    /// Running diagnostics
    Diagnostics,
    /// Analysis complete
    Complete,
    /// Analysis failed
    Failed,
}

impl ThreadSafeAnalyzer {
    pub fn new() -> Self {
        Self {
            locks: AnalysisLock::new(),
            in_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start analyzing a file
    pub fn start_analysis(&self, file_id: FileId) -> bool {
        if !self.locks.try_lock(file_id) {
            return false;
        }

        let progress = AnalysisProgress {
            file_id,
            stage: AnalysisStage::Parsing,
            progress_percent: 0,
        };

        self.in_progress.lock().unwrap().insert(file_id, progress);
        true
    }

    /// Update analysis progress
    pub fn update_progress(&self, file_id: FileId, stage: AnalysisStage, percent: u8) {
        if let Some(progress) = self.in_progress.lock().unwrap().get_mut(&file_id) {
            progress.stage = stage;
            progress.progress_percent = percent.min(100);
        }
    }

    /// Complete analysis
    pub fn complete_analysis(&self, file_id: FileId) {
        self.update_progress(file_id, AnalysisStage::Complete, 100);
        self.in_progress.lock().unwrap().remove(&file_id);
        self.locks.unlock(file_id);
    }

    /// Mark analysis as failed
    pub fn fail_analysis(&self, file_id: FileId) {
        self.update_progress(file_id, AnalysisStage::Failed, 0);
        self.in_progress.lock().unwrap().remove(&file_id);
        self.locks.unlock(file_id);
    }

    /// Get progress for file
    pub fn get_progress(&self, file_id: FileId) -> Option<AnalysisProgress> {
        self.in_progress.lock().unwrap().get(&file_id).cloned()
    }

    /// Check if file is being analyzed
    pub fn is_analyzing(&self, file_id: FileId) -> bool {
        self.in_progress.lock().unwrap().contains_key(&file_id)
    }

    /// Get all files being analyzed
    pub fn analyzing_files(&self) -> Vec<FileId> {
        self.in_progress
            .lock()
            .unwrap()
            .keys()
            .copied()
            .collect()
    }

    /// Cancel analysis for file
    pub fn cancel_analysis(&self, file_id: FileId) {
        self.in_progress.lock().unwrap().remove(&file_id);
        self.locks.unlock(file_id);
    }
}

impl Default for ThreadSafeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe result cache
pub struct ResultCache<T> {
    cache: Arc<RwLock<HashMap<FileId, T>>>,
    max_size: usize,
}

impl<T: Clone> ResultCache<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    /// Get cached result
    pub fn get(&self, file_id: FileId) -> Option<T> {
        self.cache.read().unwrap().get(&file_id).cloned()
    }

    /// Store result
    pub fn store(&self, file_id: FileId, result: T) {
        let mut cache = self.cache.write().unwrap();

        // Simple eviction: remove oldest if at capacity
        if cache.len() >= self.max_size {
            if let Some(first_key) = cache.keys().next().copied() {
                cache.remove(&first_key);
            }
        }

        cache.insert(file_id, result);
    }

    /// Invalidate cached result
    pub fn invalidate(&self, file_id: FileId) {
        self.cache.write().unwrap().remove(&file_id);
    }

    /// Clear all cached results
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_lock() {
        let lock = AnalysisLock::new();
        let file_id = FileId::new(1);

        assert!(lock.try_lock(file_id));
        assert!(lock.is_locked(file_id));
        assert!(!lock.try_lock(file_id)); // Already locked

        lock.unlock(file_id);
        assert!(!lock.is_locked(file_id));
        assert!(lock.try_lock(file_id)); // Can lock again
    }

    #[test]
    fn test_analysis_lock_count() {
        let lock = AnalysisLock::new();

        lock.try_lock(FileId::new(1));
        lock.try_lock(FileId::new(2));
        lock.try_lock(FileId::new(3));

        assert_eq!(lock.locked_count(), 3);

        lock.unlock(FileId::new(2));
        assert_eq!(lock.locked_count(), 2);
    }

    #[test]
    fn test_thread_safe_analyzer() {
        let analyzer = ThreadSafeAnalyzer::new();
        let file_id = FileId::new(1);

        assert!(analyzer.start_analysis(file_id));
        assert!(analyzer.is_analyzing(file_id));

        analyzer.update_progress(file_id, AnalysisStage::Indexing, 50);
        let progress = analyzer.get_progress(file_id).unwrap();
        assert_eq!(progress.stage, AnalysisStage::Indexing);
        assert_eq!(progress.progress_percent, 50);

        analyzer.complete_analysis(file_id);
        assert!(!analyzer.is_analyzing(file_id));
    }

    #[test]
    fn test_concurrent_analysis() {
        let analyzer = ThreadSafeAnalyzer::new();
        let file1 = FileId::new(1);
        let file2 = FileId::new(2);

        assert!(analyzer.start_analysis(file1));
        assert!(analyzer.start_analysis(file2));
        assert!(!analyzer.start_analysis(file1)); // Already analyzing

        assert_eq!(analyzer.analyzing_files().len(), 2);

        analyzer.complete_analysis(file1);
        assert_eq!(analyzer.analyzing_files().len(), 1);
    }

    #[test]
    fn test_analysis_cancellation() {
        let analyzer = ThreadSafeAnalyzer::new();
        let file_id = FileId::new(1);

        analyzer.start_analysis(file_id);
        assert!(analyzer.is_analyzing(file_id));

        analyzer.cancel_analysis(file_id);
        assert!(!analyzer.is_analyzing(file_id));
    }

    #[test]
    fn test_result_cache() {
        let cache: ResultCache<String> = ResultCache::new(100);
        let file_id = FileId::new(1);

        cache.store(file_id, "result".to_string());
        assert_eq!(cache.get(file_id), Some("result".to_string()));
        assert_eq!(cache.size(), 1);

        cache.invalidate(file_id);
        assert_eq!(cache.get(file_id), None);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_cache_eviction() {
        let cache: ResultCache<i32> = ResultCache::new(2);

        cache.store(FileId::new(1), 10);
        cache.store(FileId::new(2), 20);
        cache.store(FileId::new(3), 30); // Should trigger eviction

        assert_eq!(cache.size(), 2);
    }
}

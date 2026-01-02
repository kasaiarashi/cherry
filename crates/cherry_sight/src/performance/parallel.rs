// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Parallel processing for faster indexing

use crate::util::FileId;
use std::sync::Arc;

/// Task priority for work scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Indexing task for parallel execution
#[derive(Debug, Clone)]
pub struct IndexingTask {
    pub file_id: FileId,
    pub priority: TaskPriority,
    pub content: Arc<String>,
}

/// Parallel indexer using work-stealing
pub struct ParallelIndexer {
    #[allow(dead_code)]
    thread_count: usize,
}

impl ParallelIndexer {
    pub fn new(thread_count: usize) -> Self {
        Self { thread_count }
    }

    /// Index multiple files in parallel
    pub fn index_files(&self, _tasks: Vec<IndexingTask>) -> Vec<FileId> {
        // Would use rayon or tokio for parallel execution
        // For now, placeholder
        Vec::new()
    }

    /// Get optimal thread count
    pub fn optimal_threads() -> usize {
        num_cpus::get()
    }
}

impl Default for ParallelIndexer {
    fn default() -> Self {
        Self::new(Self::optimal_threads())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_parallel_indexer_creation() {
        let indexer = ParallelIndexer::new(4);
        assert!(indexer.thread_count >= 1);
    }

    #[test]
    fn test_optimal_threads() {
        let count = ParallelIndexer::optimal_threads();
        assert!(count >= 1);
    }
}

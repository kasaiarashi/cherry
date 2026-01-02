// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Background indexing and processing

use crate::util::FileId;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// Index request for background processing
#[derive(Debug, Clone)]
pub struct IndexRequest {
    pub file_id: FileId,
    pub content: Arc<String>,
    pub timestamp: u64,
}

/// Worker queue for background tasks
pub struct WorkerQueue {
    queue: Arc<Mutex<VecDeque<IndexRequest>>>,
}

impl WorkerQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Add request to queue
    pub fn push(&self, request: IndexRequest) {
        self.queue.lock().unwrap().push_back(request);
    }

    /// Get next request from queue
    pub fn pop(&self) -> Option<IndexRequest> {
        self.queue.lock().unwrap().pop_front()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear the queue
    pub fn clear(&self) {
        self.queue.lock().unwrap().clear();
    }
}

impl Default for WorkerQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Background worker for async indexing
pub struct BackgroundWorker {
    queue: WorkerQueue,
    #[allow(dead_code)]
    running: Arc<Mutex<bool>>,
}

impl BackgroundWorker {
    pub fn new() -> Self {
        Self {
            queue: WorkerQueue::new(),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the background worker
    pub fn start(&mut self) {
        *self.running.lock().unwrap() = true;
        // Would spawn worker thread here
    }

    /// Stop the background worker
    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
    }

    /// Submit request for background processing
    pub fn submit(&self, request: IndexRequest) {
        self.queue.push(request);
    }

    /// Get queue reference
    pub fn queue(&self) -> &WorkerQueue {
        &self.queue
    }

    /// Check if worker is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }
}

impl Default for BackgroundWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_queue() {
        let queue = WorkerQueue::new();

        let request = IndexRequest {
            file_id: FileId::new(1),
            content: Arc::new("test".to_string()),
            timestamp: 0,
        };

        queue.push(request);
        assert_eq!(queue.len(), 1);

        let popped = queue.pop();
        assert!(popped.is_some());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_background_worker() {
        let mut worker = BackgroundWorker::new();

        assert!(!worker.is_running());

        worker.start();
        assert!(worker.is_running());

        worker.stop();
        assert!(!worker.is_running());
    }

    #[test]
    fn test_worker_submit() {
        let worker = BackgroundWorker::new();

        let request = IndexRequest {
            file_id: FileId::new(1),
            content: Arc::new("content".to_string()),
            timestamp: 100,
        };

        worker.submit(request);
        assert_eq!(worker.queue().len(), 1);
    }
}

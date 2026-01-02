// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Performance and scalability optimizations

mod parallel;
mod memory;
mod background;

pub use parallel::{ParallelIndexer, IndexingTask, TaskPriority};
pub use memory::{MemoryPool, CacheManager, CacheStrategy};
pub use background::{BackgroundWorker, WorkerQueue, IndexRequest};

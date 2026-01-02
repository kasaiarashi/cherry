// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Memory optimization and caching

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cache eviction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// First In First Out
    Fifo,
}

/// Memory pool for reusing allocations
pub struct MemoryPool<T> {
    #[allow(dead_code)]
    pool: Vec<T>,
    #[allow(dead_code)]
    capacity: usize,
}

impl<T> MemoryPool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Acquire an item from the pool
    pub fn acquire(&mut self) -> Option<T> {
        self.pool.pop()
    }

    /// Return an item to the pool
    pub fn release(&mut self, item: T) {
        if self.pool.len() < self.capacity {
            self.pool.push(item);
        }
    }
}

/// Generic cache manager
pub struct CacheManager<K, V> {
    cache: Arc<RwLock<HashMap<K, V>>>,
    max_size: usize,
    strategy: CacheStrategy,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> CacheManager<K, V> {
    pub fn new(max_size: usize, strategy: CacheStrategy) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            strategy,
        }
    }

    /// Get value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        self.cache.read().ok()?.get(key).cloned()
    }

    /// Insert value into cache
    pub fn insert(&mut self, key: K, value: V) {
        let mut cache = self.cache.write().unwrap();
        if cache.len() >= self.max_size {
            // Would implement actual eviction based on strategy
            // For now, just clear oldest entry
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }
        cache.insert(key, value);
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.write().unwrap().clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.size(),
            max_size: self.max_size,
            strategy: self.strategy,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub strategy: CacheStrategy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        let mut pool: MemoryPool<String> = MemoryPool::new(10);

        pool.release("test".to_string());
        assert!(pool.acquire().is_some());
        assert!(pool.acquire().is_none());
    }

    #[test]
    fn test_cache_manager() {
        let mut cache: CacheManager<String, i32> = CacheManager::new(100, CacheStrategy::Lru);

        cache.insert("key1".to_string(), 42);
        assert_eq!(cache.get(&"key1".to_string()), Some(42));
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache: CacheManager<i32, i32> = CacheManager::new(2, CacheStrategy::Lru);

        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30); // Should trigger eviction

        assert_eq!(cache.size(), 2);
    }
}

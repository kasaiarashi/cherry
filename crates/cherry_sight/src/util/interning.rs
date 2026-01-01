// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! String interning using lasso for efficient memory usage

use lasso::{Rodeo, Spur, ThreadedRodeo};
use std::sync::Arc;
use parking_lot::RwLock;

/// Interned string handle - a lightweight reference to an interned string
pub type InternedString = Spur;

/// Thread-safe string interner
#[derive(Debug, Clone)]
pub struct Interner {
    rodeo: Arc<RwLock<Rodeo>>,
}

impl Interner {
    /// Create a new interner
    pub fn new() -> Self {
        Self {
            rodeo: Arc::new(RwLock::new(Rodeo::new())),
        }
    }

    /// Intern a string, returning its handle
    pub fn intern(&self, s: &str) -> InternedString {
        self.rodeo.write().get_or_intern(s)
    }

    /// Resolve an interned string back to its value
    pub fn resolve(&self, handle: InternedString) -> String {
        self.rodeo.read().resolve(&handle).to_string()
    }

    /// Try to find an existing interned string without interning
    pub fn get(&self, s: &str) -> Option<InternedString> {
        self.rodeo.read().get(s)
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe interner that can be used across threads
#[derive(Debug, Clone)]
pub struct ThreadSafeInterner {
    rodeo: Arc<ThreadedRodeo>,
}

impl ThreadSafeInterner {
    /// Create a new thread-safe interner
    pub fn new() -> Self {
        Self {
            rodeo: Arc::new(ThreadedRodeo::new()),
        }
    }

    /// Intern a string, returning its handle
    pub fn intern(&self, s: &str) -> InternedString {
        self.rodeo.get_or_intern(s)
    }

    /// Resolve an interned string back to its value
    pub fn resolve(&self, handle: InternedString) -> String {
        self.rodeo.resolve(&handle).to_string()
    }

    /// Try to find an existing interned string without interning
    pub fn get(&self, s: &str) -> Option<InternedString> {
        self.rodeo.get(s)
    }
}

impl Default for ThreadSafeInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interner_basic() {
        let interner = Interner::new();

        let hello = interner.intern("hello");
        let world = interner.intern("world");
        let hello2 = interner.intern("hello");

        assert_eq!(hello, hello2);
        assert_ne!(hello, world);

        assert_eq!(interner.resolve(hello), "hello");
        assert_eq!(interner.resolve(world), "world");
    }

    #[test]
    fn test_interner_get() {
        let interner = Interner::new();

        assert_eq!(interner.get("nonexistent"), None);

        let hello = interner.intern("hello");
        assert_eq!(interner.get("hello"), Some(hello));
    }

    #[test]
    fn test_thread_safe_interner() {
        let interner = ThreadSafeInterner::new();

        let hello = interner.intern("hello");
        let world = interner.intern("world");

        assert_ne!(hello, world);
        assert_eq!(interner.resolve(hello), "hello");
    }
}

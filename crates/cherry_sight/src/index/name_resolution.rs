// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Name resolution for qualified and unqualified names
//!
//! To be implemented in Phase 2

use crate::index::symbol::SymbolId;
use crate::index::scope::Scope;
use crate::util::InternedString;

/// Name resolution context
pub struct NameResolver {
    current_scope: Scope,
}

impl NameResolver {
    pub fn new() -> Self {
        Self {
            current_scope: Scope::new(),
        }
    }

    pub fn resolve_unqualified(&self, name: InternedString) -> Option<SymbolId> {
        self.current_scope.lookup(name)
    }

    pub fn resolve_qualified(&self, _path: &[InternedString]) -> Option<SymbolId> {
        // To be implemented in Phase 2
        None
    }
}

impl Default for NameResolver {
    fn default() -> Self {
        Self::new()
    }
}

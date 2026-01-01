// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Scope management for name resolution
//!
//! To be implemented in Phase 2

use crate::index::symbol::SymbolId;
use crate::util::InternedString;
use std::collections::HashMap;

/// A lexical scope
#[derive(Debug, Default)]
pub struct Scope {
    /// Symbols defined in this scope
    pub symbols: HashMap<InternedString, SymbolId>,
    /// Parent scope
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: InternedString, symbol: SymbolId) {
        self.symbols.insert(name, symbol);
    }

    pub fn lookup(&self, name: InternedString) -> Option<SymbolId> {
        self.symbols.get(&name).copied().or_else(|| {
            self.parent.as_ref().and_then(|p| p.lookup(name))
        })
    }
}

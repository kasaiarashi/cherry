// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol table for hierarchical symbol storage
//!
//! To be implemented in Phase 2

use crate::index::symbol::{Symbol, SymbolId};
use std::collections::HashMap;

/// Hierarchical symbol table
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: HashMap<SymbolId, Symbol>,
    next_id: u32,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_symbol(&mut self, symbol: Symbol) -> SymbolId {
        let id = symbol.id;
        self.symbols.insert(id, symbol);
        id
    }

    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    pub fn next_id(&mut self) -> SymbolId {
        let id = SymbolId::new(self.next_id);
        self.next_id += 1;
        id
    }
}

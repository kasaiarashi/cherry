// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Scope management for name resolution

use crate::index::symbol::SymbolId;
use crate::util::InternedString;
use rustc_hash::FxHashMap;

/// A lexical scope
#[derive(Debug, Clone)]
pub struct Scope {
    /// Symbols defined in this scope
    pub symbols: FxHashMap<InternedString, Vec<SymbolId>>,
    /// Parent scope
    pub parent: Option<SymbolId>,
    /// Scope kind
    pub kind: ScopeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Namespace,
    Class,
    Function,
    Block,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self {
            symbols: FxHashMap::default(),
            parent: None,
            kind,
        }
    }

    pub fn with_parent(kind: ScopeKind, parent: SymbolId) -> Self {
        Self {
            symbols: FxHashMap::default(),
            parent: Some(parent),
            kind,
        }
    }

    pub fn define(&mut self, name: InternedString, symbol: SymbolId) {
        self.symbols.entry(name).or_default().push(symbol);
    }

    pub fn lookup_local(&self, name: InternedString) -> Option<&Vec<SymbolId>> {
        self.symbols.get(&name)
    }
}

/// Scope stack for tracking current scope during analysis
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new(ScopeKind::Global)],
        }
    }

    pub fn push(&mut self, scope: Scope) {
        self.scopes.push(scope);
    }

    pub fn pop(&mut self) -> Option<Scope> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }

    pub fn current(&self) -> &Scope {
        self.scopes.last().unwrap()
    }

    pub fn current_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    pub fn lookup(&self, name: InternedString) -> Vec<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbols) = scope.lookup_local(name) {
                return symbols.clone();
            }
        }
        Vec::new()
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

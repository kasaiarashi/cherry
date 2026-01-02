// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Scope hierarchy for name resolution

use crate::index::SymbolId;
use crate::util::{FileId, InternedString};
use std::collections::HashMap;

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

/// Kind of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Namespace,
    Class,
    Function,
    Block,
}

/// A lexical scope that contains name bindings
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub file_id: FileId,

    /// Local name bindings in this scope
    pub bindings: HashMap<InternedString, SymbolId>,

    /// Child scopes
    pub children: Vec<ScopeId>,

    /// Associated symbol (for namespace, class, function scopes)
    pub symbol: Option<SymbolId>,
}

impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>, file_id: FileId) -> Self {
        Self {
            id,
            kind,
            parent,
            file_id,
            bindings: HashMap::new(),
            children: Vec::new(),
            symbol: None,
        }
    }

    /// Add a name binding to this scope
    pub fn add_binding(&mut self, name: InternedString, symbol: SymbolId) {
        self.bindings.insert(name, symbol);
    }

    /// Lookup a name in this scope only (not parent scopes)
    pub fn lookup_local(&self, name: InternedString) -> Option<SymbolId> {
        self.bindings.get(&name).copied()
    }
}

/// Scope manager that tracks all scopes in a file
#[derive(Debug, Clone)]
pub struct ScopeManager {
    scopes: Vec<Scope>,
    next_id: u32,
}

impl ScopeManager {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a new scope
    pub fn create_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        file_id: FileId,
    ) -> ScopeId {
        let id = ScopeId(self.next_id);
        self.next_id += 1;

        let scope = Scope::new(id, kind, parent, file_id);
        self.scopes.push(scope);

        // Add this scope as a child of its parent
        if let Some(parent_id) = parent {
            if let Some(parent_scope) = self.get_scope_mut(parent_id) {
                parent_scope.children.push(id);
            }
        }

        id
    }

    /// Get a scope by ID
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.iter().find(|s| s.id == id)
    }

    /// Get a mutable scope by ID
    pub fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.iter_mut().find(|s| s.id == id)
    }

    /// Lookup a name starting from a given scope, walking up the parent chain
    pub fn lookup(&self, scope_id: ScopeId, name: InternedString) -> Option<SymbolId> {
        let mut current = Some(scope_id);

        while let Some(scope_id) = current {
            if let Some(scope) = self.get_scope(scope_id) {
                if let Some(symbol) = scope.lookup_local(name) {
                    return Some(symbol);
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Add a binding to a scope
    pub fn add_binding(&mut self, scope_id: ScopeId, name: InternedString, symbol: SymbolId) {
        if let Some(scope) = self.get_scope_mut(scope_id) {
            scope.add_binding(name, symbol);
        }
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

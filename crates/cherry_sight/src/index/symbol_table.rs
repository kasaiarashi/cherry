// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol table for hierarchical symbol storage and lookup

use crate::index::symbol::{Symbol, SymbolId};
use crate::util::{FileId, InternedString};
use rustc_hash::FxHashMap;
use std::collections::HashMap;

/// Hierarchical symbol table
#[derive(Debug)]
pub struct SymbolTable {
    /// All symbols indexed by ID
    symbols: FxHashMap<SymbolId, Symbol>,

    /// Name-to-symbol mapping for quick lookup
    /// Key: (name, parent_scope)
    name_index: HashMap<(InternedString, Option<SymbolId>), Vec<SymbolId>>,

    /// File-to-symbols mapping
    file_index: HashMap<FileId, Vec<SymbolId>>,

    /// Type symbols for quick type lookup
    type_symbols: Vec<SymbolId>,

    /// Global symbols (no parent)
    global_symbols: Vec<SymbolId>,

    /// Next available symbol ID
    next_id: u32,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: FxHashMap::default(),
            name_index: HashMap::new(),
            file_index: HashMap::new(),
            type_symbols: Vec::new(),
            global_symbols: Vec::new(),
            next_id: 0,
        }
    }

    /// Allocate a new symbol ID
    pub fn next_id(&mut self) -> SymbolId {
        let id = SymbolId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a symbol to the table
    pub fn add_symbol(&mut self, symbol: Symbol) -> SymbolId {
        let id = symbol.id;
        let name = symbol.name;
        let parent = symbol.parent;
        let file_id = symbol.file_id;

        // Update name index
        let key = (name, parent);
        self.name_index.entry(key).or_default().push(id);

        // Update file index
        self.file_index.entry(file_id).or_default().push(id);

        // Track type symbols
        if symbol.kind.is_type() {
            self.type_symbols.push(id);
        }

        // Track global symbols
        if symbol.parent.is_none() {
            self.global_symbols.push(id);
        }

        // Store the symbol
        self.symbols.insert(id, symbol);

        id
    }

    /// Get a symbol by ID
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    /// Get a mutable reference to a symbol
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(&id)
    }

    /// Find symbols by name in a given scope
    pub fn find_in_scope(
        &self,
        name: InternedString,
        scope: Option<SymbolId>,
    ) -> Vec<SymbolId> {
        let key = (name, scope);
        self.name_index.get(&key).cloned().unwrap_or_default()
    }

    /// Find all symbols with a given name (across all scopes)
    pub fn find_all(&self, name: InternedString) -> Vec<SymbolId> {
        let mut results = Vec::new();
        for ((sym_name, _), ids) in &self.name_index {
            if *sym_name == name {
                results.extend(ids);
            }
        }
        results
    }

    /// Get all symbols in a file
    pub fn symbols_in_file(&self, file_id: FileId) -> Vec<SymbolId> {
        self.file_index.get(&file_id).cloned().unwrap_or_default()
    }

    /// Get all type symbols
    pub fn type_symbols(&self) -> &[SymbolId] {
        &self.type_symbols
    }

    /// Get all global symbols
    pub fn global_symbols(&self) -> &[SymbolId] {
        &self.global_symbols
    }

    /// Get all symbol IDs
    pub fn all_symbols(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.symbols.keys().copied()
    }

    /// Get children of a symbol
    pub fn children(&self, parent_id: SymbolId) -> Vec<SymbolId> {
        self.symbols
            .get(&parent_id)
            .map(|s| s.children.clone())
            .unwrap_or_default()
    }

    /// Get the parent of a symbol
    pub fn parent(&self, id: SymbolId) -> Option<SymbolId> {
        self.symbols.get(&id).and_then(|s| s.parent)
    }

    /// Get the scope chain from a symbol to the root
    pub fn scope_chain(&self, id: SymbolId) -> Vec<SymbolId> {
        let mut chain = vec![id];
        let mut current = id;

        while let Some(parent_id) = self.parent(current) {
            chain.push(parent_id);
            current = parent_id;
        }

        chain.reverse();
        chain
    }

    /// Find all derived classes of a base class
    pub fn find_derived(&self, base_id: SymbolId) -> Vec<SymbolId> {
        let mut derived = Vec::new();

        if let Some(base) = self.get_symbol(base_id) {
            derived.extend(&base.derived);

            // Recursively find all transitive derived classes
            let direct_derived = base.derived.clone();
            for derived_id in direct_derived {
                derived.extend(self.find_derived(derived_id));
            }
        }

        derived
    }

    /// Find all base classes of a derived class
    pub fn find_bases(&self, derived_id: SymbolId) -> Vec<SymbolId> {
        let mut bases = Vec::new();

        if let Some(derived) = self.get_symbol(derived_id) {
            bases.extend(&derived.bases);

            // Recursively find all transitive base classes
            let direct_bases = derived.bases.clone();
            for base_id in direct_bases {
                bases.extend(self.find_bases(base_id));
            }
        }

        bases
    }

    /// Check if a class inherits from another (directly or indirectly)
    pub fn inherits_from(&self, derived_id: SymbolId, base_id: SymbolId) -> bool {
        if derived_id == base_id {
            return true;
        }

        if let Some(derived) = self.get_symbol(derived_id) {
            for direct_base in &derived.bases {
                if self.inherits_from(*direct_base, base_id) {
                    return true;
                }
            }
        }

        false
    }

    /// Find all overrides of a virtual method
    pub fn find_overrides(&self, method_id: SymbolId) -> Vec<SymbolId> {
        self.get_symbol(method_id)
            .map(|s| s.overridden_by.clone())
            .unwrap_or_default()
    }

    /// Find the method this symbol overrides
    pub fn find_overridden(&self, method_id: SymbolId) -> Option<SymbolId> {
        self.get_symbol(method_id).and_then(|s| s.overrides)
    }

    /// Get total number of symbols
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Clear all symbols
    pub fn clear(&mut self) {
        self.symbols.clear();
        self.name_index.clear();
        self.file_index.clear();
        self.type_symbols.clear();
        self.global_symbols.clear();
        self.next_id = 0;
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::SymbolKind;
    use crate::util::{Interner, Span};

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let name = interner.intern("MyClass");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, span, file_id);

        table.add_symbol(symbol);

        assert_eq!(table.len(), 1);
        assert!(table.get_symbol(id).is_some());
    }

    #[test]
    fn test_symbol_lookup_by_name() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let name = interner.intern("MyClass");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, span, file_id);

        table.add_symbol(symbol);

        let found = table.find_in_scope(name, None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], id);
    }

    #[test]
    fn test_symbol_hierarchy() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let parent_name = interner.intern("Namespace");
        let child_name = interner.intern("Class");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        // Create parent
        let parent_id = table.next_id();
        let mut parent = Symbol::new(parent_id, SymbolKind::Namespace, parent_name, span, file_id);

        // Create child
        let child_id = table.next_id();
        let mut child = Symbol::new(child_id, SymbolKind::Class, child_name, span, file_id);
        child.parent = Some(parent_id);

        parent.add_child(child_id);

        table.add_symbol(parent);
        table.add_symbol(child);

        assert_eq!(table.children(parent_id), vec![child_id]);
        assert_eq!(table.parent(child_id), Some(parent_id));
    }

    #[test]
    fn test_inheritance_queries() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let base_name = interner.intern("Base");
        let derived_name = interner.intern("Derived");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        // Create base class
        let base_id = table.next_id();
        let mut base = Symbol::new(base_id, SymbolKind::Class, base_name, span, file_id);

        // Create derived class
        let derived_id = table.next_id();
        let mut derived = Symbol::new(derived_id, SymbolKind::Class, derived_name, span, file_id);

        derived.add_base(base_id);
        base.add_derived(derived_id);

        table.add_symbol(base);
        table.add_symbol(derived);

        assert!(table.inherits_from(derived_id, base_id));
        assert!(!table.inherits_from(base_id, derived_id));

        let bases = table.find_bases(derived_id);
        assert!(bases.contains(&base_id));

        let derived_classes = table.find_derived(base_id);
        assert!(derived_classes.contains(&derived_id));
    }

    #[test]
    fn test_file_index() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file1 = FileId::new(1);
        let file2 = FileId::new(2);
        let span = Span::new(file1, 0, 10);

        let name1 = interner.intern("Class1");
        let name2 = interner.intern("Class2");

        let id1 = table.next_id();
        let symbol1 = Symbol::new(id1, SymbolKind::Class, name1, span, file1);

        let id2 = table.next_id();
        let symbol2 = Symbol::new(id2, SymbolKind::Class, name2, Span::new(file2, 0, 10), file2);

        table.add_symbol(symbol1);
        table.add_symbol(symbol2);

        let file1_symbols = table.symbols_in_file(file1);
        assert_eq!(file1_symbols.len(), 1);
        assert_eq!(file1_symbols[0], id1);

        let file2_symbols = table.symbols_in_file(file2);
        assert_eq!(file2_symbols.len(), 1);
        assert_eq!(file2_symbols[0], id2);
    }

    #[test]
    fn test_scope_chain() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        // Create: Namespace -> Class -> Method
        let ns_id = table.next_id();
        let mut ns = Symbol::new(
            ns_id,
            SymbolKind::Namespace,
            interner.intern("NS"),
            span,
            file_id,
        );

        let class_id = table.next_id();
        let mut class = Symbol::new(
            class_id,
            SymbolKind::Class,
            interner.intern("Class"),
            span,
            file_id,
        );
        class.parent = Some(ns_id);
        ns.add_child(class_id);

        let method_id = table.next_id();
        let mut method = Symbol::new(
            method_id,
            SymbolKind::Method,
            interner.intern("Method"),
            span,
            file_id,
        );
        method.parent = Some(class_id);
        class.add_child(method_id);

        table.add_symbol(ns);
        table.add_symbol(class);
        table.add_symbol(method);

        let chain = table.scope_chain(method_id);
        assert_eq!(chain, vec![ns_id, class_id, method_id]);
    }
}

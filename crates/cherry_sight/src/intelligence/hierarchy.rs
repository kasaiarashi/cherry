// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Type hierarchy and call hierarchy

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, InternedString, Span};

/// Type hierarchy node
#[derive(Debug, Clone)]
pub struct TypeHierarchyNode {
    pub symbol_id: SymbolId,
    pub name: InternedString,
    pub kind: SymbolKind,
    pub span: Span,
    pub file_id: FileId,
    pub children: Vec<TypeHierarchyNode>,
}

/// Call hierarchy node
#[derive(Debug, Clone)]
pub struct CallHierarchyNode {
    pub symbol_id: SymbolId,
    pub name: InternedString,
    pub span: Span,
    pub file_id: FileId,
    pub call_kind: CallKind,
}

/// Kind of call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// Function calls this symbol
    IncomingCall,
    /// Symbol calls this function
    OutgoingCall,
}

/// Builds type hierarchies
pub struct TypeHierarchyBuilder<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> TypeHierarchyBuilder<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Build a type hierarchy tree for a class/struct
    pub fn build_hierarchy(&self, symbol_id: SymbolId) -> Option<TypeHierarchyNode> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;

        if !matches!(
            symbol.kind,
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::UClass | SymbolKind::UStruct
        ) {
            return None;
        }

        self.build_node(symbol_id)
    }

    /// Build a subtree showing all base classes
    pub fn build_supertypes(&self, symbol_id: SymbolId) -> Vec<TypeHierarchyNode> {
        let mut supertypes = Vec::new();

        // Get direct base classes
        if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
            for &base_id in &symbol.bases {
                if let Some(node) = self.build_node(base_id) {
                    supertypes.push(node);
                }
            }
        }

        supertypes
    }

    /// Build a subtree showing all derived classes
    pub fn build_subtypes(&self, symbol_id: SymbolId) -> Vec<TypeHierarchyNode> {
        let mut subtypes = Vec::new();

        let derived_ids = self.symbol_table.find_derived(symbol_id);
        for &derived_id in &derived_ids {
            if let Some(node) = self.build_node(derived_id) {
                subtypes.push(node);
            }
        }

        subtypes
    }

    /// Build a complete hierarchy tree (both up and down)
    pub fn build_complete_hierarchy(&self, symbol_id: SymbolId) -> Option<TypeHierarchyNode> {
        // Find the root (topmost base class)
        let mut root_id = symbol_id;
        while let Some(symbol) = self.symbol_table.get_symbol(root_id) {
            if symbol.bases.is_empty() {
                break;
            }
            root_id = symbol.bases[0]; // Take first base for simplicity
        }

        // Build tree from root
        self.build_tree_recursive(root_id)
    }

    /// Build a hierarchy node
    fn build_node(&self, symbol_id: SymbolId) -> Option<TypeHierarchyNode> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;

        Some(TypeHierarchyNode {
            symbol_id,
            name: symbol.name,
            kind: symbol.kind,
            span: symbol.span,
            file_id: symbol.file_id,
            children: Vec::new(),
        })
    }

    /// Recursively build hierarchy tree
    fn build_tree_recursive(&self, symbol_id: SymbolId) -> Option<TypeHierarchyNode> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;

        let mut children = Vec::new();

        // Add derived classes as children
        for &derived_id in &symbol.derived {
            if let Some(child_node) = self.build_tree_recursive(derived_id) {
                children.push(child_node);
            }
        }

        Some(TypeHierarchyNode {
            symbol_id,
            name: symbol.name,
            kind: symbol.kind,
            span: symbol.span,
            file_id: symbol.file_id,
            children,
        })
    }
}

/// Builds call hierarchies
pub struct CallHierarchyBuilder<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> CallHierarchyBuilder<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Find all incoming calls (functions that call this symbol)
    pub fn find_incoming_calls(&self, symbol_id: SymbolId) -> Vec<CallHierarchyNode> {
        let calls = Vec::new();

        let _symbol = match self.symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return calls,
        };

        // In a real implementation, we would:
        // 1. Parse all functions
        // 2. Track function call expressions
        // 3. Resolve call targets
        // 4. Build incoming call graph

        // This is a placeholder showing the structure
        // We would need AST analysis to find actual call sites

        calls
    }

    /// Find all outgoing calls (functions this symbol calls)
    pub fn find_outgoing_calls(&self, symbol_id: SymbolId) -> Vec<CallHierarchyNode> {
        let calls = Vec::new();

        let _symbol = match self.symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return calls,
        };

        // In a real implementation, we would:
        // 1. Get the function body AST
        // 2. Find all call expressions
        // 3. Resolve call targets
        // 4. Build outgoing call list

        // This is a placeholder showing the structure

        calls
    }

    /// Build a call hierarchy tree (recursive incoming calls)
    pub fn build_incoming_tree(&self, symbol_id: SymbolId, max_depth: usize) -> Vec<CallHierarchyNode> {
        if max_depth == 0 {
            return Vec::new();
        }

        self.find_incoming_calls(symbol_id)
        // Would recursively build tree for each caller
    }

    /// Build a call hierarchy tree (recursive outgoing calls)
    pub fn build_outgoing_tree(&self, symbol_id: SymbolId, max_depth: usize) -> Vec<CallHierarchyNode> {
        if max_depth == 0 {
            return Vec::new();
        }

        self.find_outgoing_calls(symbol_id)
        // Would recursively build tree for each callee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Interner;

    #[test]
    fn test_build_hierarchy() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let base_name = interner.intern("Base");
        let derived_name = interner.intern("Derived");

        // Create base class
        let base_id = table.next_id();
        let mut base = Symbol::new(base_id, SymbolKind::Class, base_name, Span::new(file_id, 0, 10), file_id);

        // Create derived class
        let derived_id = table.next_id();
        let mut derived = Symbol::new(
            derived_id,
            SymbolKind::Class,
            derived_name,
            Span::new(file_id, 20, 30),
            file_id,
        );
        derived.add_base(base_id);
        base.add_derived(derived_id);

        table.add_symbol(base);
        table.add_symbol(derived);

        let builder = TypeHierarchyBuilder::new(&table);
        let hierarchy = builder.build_hierarchy(base_id);

        assert!(hierarchy.is_some());
        let node = hierarchy.unwrap();
        assert_eq!(node.symbol_id, base_id);
    }

    #[test]
    fn test_build_subtypes() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let base_name = interner.intern("Base");
        let derived_name = interner.intern("Derived");

        // Create base class
        let base_id = table.next_id();
        let mut base = Symbol::new(base_id, SymbolKind::Class, base_name, Span::new(file_id, 0, 10), file_id);

        // Create derived class
        let derived_id = table.next_id();
        let mut derived = Symbol::new(
            derived_id,
            SymbolKind::Class,
            derived_name,
            Span::new(file_id, 20, 30),
            file_id,
        );
        derived.add_base(base_id);
        base.add_derived(derived_id);

        table.add_symbol(base);
        table.add_symbol(derived);

        let builder = TypeHierarchyBuilder::new(&table);
        let subtypes = builder.build_subtypes(base_id);

        assert_eq!(subtypes.len(), 1);
        assert_eq!(subtypes[0].symbol_id, derived_id);
    }

    #[test]
    fn test_call_hierarchy() {
        let table = SymbolTable::new();
        let builder = CallHierarchyBuilder::new(&table);

        // Test structure - actual implementation needs AST analysis
        let incoming = builder.find_incoming_calls(SymbolId::new(0));
        assert_eq!(incoming.len(), 0); // No calls tracked yet
    }
}

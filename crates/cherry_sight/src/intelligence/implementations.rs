// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Find implementations of virtual methods and interfaces

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Span};

/// Result of an implementation search
#[derive(Debug, Clone)]
pub struct ImplementationResult {
    pub symbol_id: SymbolId,
    pub span: Span,
    pub file_id: FileId,
    pub kind: ImplementationKind,
}

/// Kind of implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationKind {
    /// Class implementing an interface
    InterfaceImplementation,
    /// Method overriding a virtual method
    MethodOverride,
    /// Derived class
    DerivedClass,
}

/// Finds implementations
pub struct ImplementationFinder<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> ImplementationFinder<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Find all implementations of a symbol
    pub fn find_implementations(&self, symbol_id: SymbolId) -> Vec<ImplementationResult> {
        let mut results = Vec::new();

        let symbol = match self.symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return results,
        };

        match symbol.kind {
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::UClass | SymbolKind::UStruct => {
                // Find all derived classes
                self.find_derived_classes(symbol_id, &mut results);
            }
            SymbolKind::Method => {
                // Find all method overrides
                self.find_method_overrides(symbol_id, &mut results);
            }
            _ => {}
        }

        results
    }

    /// Find all classes that derive from the given class
    fn find_derived_classes(&self, class_id: SymbolId, results: &mut Vec<ImplementationResult>) {
        let derived_ids = self.symbol_table.find_derived(class_id);

        for &derived_id in &derived_ids {
            if let Some(derived) = self.symbol_table.get_symbol(derived_id) {
                results.push(ImplementationResult {
                    symbol_id: derived_id,
                    span: derived.span,
                    file_id: derived.file_id,
                    kind: ImplementationKind::DerivedClass,
                });
            }
        }
    }

    /// Find all methods that override the given method
    fn find_method_overrides(&self, method_id: SymbolId, results: &mut Vec<ImplementationResult>) {
        let overrides = self.symbol_table.find_overrides(method_id);

        for &override_id in &overrides {
            if let Some(override_method) = self.symbol_table.get_symbol(override_id) {
                results.push(ImplementationResult {
                    symbol_id: override_id,
                    span: override_method.span,
                    file_id: override_method.file_id,
                    kind: ImplementationKind::MethodOverride,
                });
            }
        }

        // Recursively find overrides of overrides
        for &override_id in &overrides {
            self.find_method_overrides(override_id, results);
        }
    }

    /// Find the base method that this method implements
    pub fn find_base_implementation(&self, method_id: SymbolId) -> Option<ImplementationResult> {
        let method = self.symbol_table.get_symbol(method_id)?;

        if method.kind != SymbolKind::Method {
            return None;
        }

        // Check if this method overrides another
        let base_id = self.symbol_table.find_overridden(method_id)?;
        let base_method = self.symbol_table.get_symbol(base_id)?;

        Some(ImplementationResult {
            symbol_id: base_id,
            span: base_method.span,
            file_id: base_method.file_id,
            kind: ImplementationKind::MethodOverride,
        })
    }

    /// Find all implementations in the entire hierarchy (both up and down)
    pub fn find_all_in_hierarchy(&self, method_id: SymbolId) -> Vec<ImplementationResult> {
        let mut results = Vec::new();

        // Find the topmost base method
        let mut current_id = method_id;
        while let Some(base_id) = self.symbol_table.find_overridden(current_id) {
            current_id = base_id;
        }
        let root_id = current_id;

        // Include the root
        if let Some(root) = self.symbol_table.get_symbol(root_id) {
            results.push(ImplementationResult {
                symbol_id: root_id,
                span: root.span,
                file_id: root.file_id,
                kind: ImplementationKind::MethodOverride,
            });
        }

        // Find all overrides starting from root
        self.find_method_overrides(root_id, &mut results);

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::{Interner, Span};

    #[test]
    fn test_find_derived_classes() {
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

        let finder = ImplementationFinder::new(&table);
        let results = finder.find_implementations(base_id);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_id, derived_id);
        assert_eq!(results[0].kind, ImplementationKind::DerivedClass);
    }

    #[test]
    fn test_find_method_overrides() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let method_name = interner.intern("Update");

        // Create base method
        let base_id = table.next_id();
        let mut base_method = Symbol::new(
            base_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 0, 10),
            file_id,
        );

        // Create override
        let override_id = table.next_id();
        let mut override_method = Symbol::new(
            override_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 20, 30),
            file_id,
        );
        override_method.overrides = Some(base_id);
        base_method.add_overridden_by(override_id);

        table.add_symbol(base_method);
        table.add_symbol(override_method);

        let finder = ImplementationFinder::new(&table);
        let results = finder.find_implementations(base_id);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_id, override_id);
        assert_eq!(results[0].kind, ImplementationKind::MethodOverride);
    }

    #[test]
    fn test_find_base_implementation() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let method_name = interner.intern("Update");

        // Create base method
        let base_id = table.next_id();
        let mut base_method = Symbol::new(
            base_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 0, 10),
            file_id,
        );

        // Create override
        let override_id = table.next_id();
        let mut override_method = Symbol::new(
            override_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 20, 30),
            file_id,
        );
        override_method.overrides = Some(base_id);
        base_method.add_overridden_by(override_id);

        table.add_symbol(base_method);
        table.add_symbol(override_method);

        let finder = ImplementationFinder::new(&table);
        let result = finder.find_base_implementation(override_id);

        assert!(result.is_some());
        let base_impl = result.unwrap();
        assert_eq!(base_impl.symbol_id, base_id);
    }
}

// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Interface implementation generation

use crate::index::symbol::SymbolId;
use crate::index::symbol_table::SymbolTable;
use crate::util::Interner;

/// Implements interface methods
pub struct InterfaceImplementor<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> InterfaceImplementor<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Generate stub implementations for all interface methods
    pub fn implement_interface(&self, _interface_id: SymbolId) -> Option<String> {
        // Would:
        // 1. Get all pure virtual methods from interface
        // 2. Generate stub implementations
        // 3. Return formatted code

        Some(String::new())
    }

    /// Generate override for a virtual method
    pub fn override_method(&self, method_id: SymbolId) -> Option<String> {
        let method = self.symbol_table.get_symbol(method_id)?;
        let method_name = self.interner.resolve(method.name);

        // Simple stub
        let mut code = format!("virtual void {}() override {{\n", method_name);
        code.push_str("    // TODO: Implement\n");
        code.push_str("}\n");

        Some(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::{Symbol, SymbolKind};
    use crate::util::{FileId, Span};

    #[test]
    fn test_override_method() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("Update");
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Method, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let implementor = InterfaceImplementor::new(&table, &interner);
        let code = implementor.override_method(id);

        assert!(code.is_some());
        assert!(code.unwrap().contains("Update"));
    }
}

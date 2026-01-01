// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Function signature help

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner, Position};

/// Signature help information
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: Option<usize>,
    pub active_parameter: Option<usize>,
}

/// Information about a function signature
#[derive(Debug, Clone)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformation>,
}

/// Information about a parameter
#[derive(Debug, Clone)]
pub struct ParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

/// Provides signature help
pub struct SignatureHelpProvider<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> SignatureHelpProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get signature help at a position
    pub fn get_signature_help(
        &self,
        _file_id: FileId,
        _position: Position,
    ) -> Option<SignatureHelp> {
        // Would need to:
        // 1. Find the function call expression at position
        // 2. Identify the function being called
        // 3. Find all overloads
        // 4. Determine active parameter from cursor position

        // Placeholder implementation
        None
    }

    /// Create signature information from a function symbol
    fn create_signature_info(&self, function_id: SymbolId) -> Option<SignatureInformation> {
        let symbol = self.symbol_table.get_symbol(function_id)?;

        if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method | SymbolKind::UFunction) {
            return None;
        }

        let name = self.interner.resolve(symbol.name);

        // Would need type information to build parameter list
        // For now, return basic signature
        Some(SignatureInformation {
            label: format!("{}(...)", name),
            documentation: symbol.doc_comment.clone(),
            parameters: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::Span;

    #[test]
    fn test_signature_info_creation() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyFunction");
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Function, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let provider = SignatureHelpProvider::new(&table, &interner);
        let sig = provider.create_signature_info(id);

        assert!(sig.is_some());
        let sig_info = sig.unwrap();
        assert_eq!(sig_info.label, "MyFunction(...)");
    }
}

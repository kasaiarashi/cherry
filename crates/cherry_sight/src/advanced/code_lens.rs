// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Code lens for actionable insights

use crate::index::symbol::SymbolId;
use crate::index::SymbolTable;
use crate::util::{FileId, Interner, Span};

/// Kind of code lens
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeLensKind {
    /// Reference count (e.g., "5 references")
    References,
    /// Implementation count (e.g., "3 implementations")
    Implementations,
    /// Test status (e.g., "Run test", "Debug test")
    Test,
    /// UE5-specific (e.g., "Open in Blueprint")
    UE5Action,
    /// Code metrics (e.g., "Complexity: 15")
    Metrics,
}

/// Code lens with action
#[derive(Debug, Clone)]
pub struct CodeLens {
    pub span: Span,
    pub kind: CodeLensKind,
    pub title: String,
    pub command: Option<String>,
    pub arguments: Vec<String>,
}

/// Provides code lens
pub struct CodeLensProvider<'a> {
    #[allow(dead_code)]
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> CodeLensProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get all code lenses for a file
    pub fn get_code_lenses(&self, _file_id: FileId) -> Vec<CodeLens> {
        // Would need:
        // 1. File symbol iteration (not yet implemented in SymbolTable)
        // 2. Reference counting infrastructure
        // 3. Implementation finding for virtual methods
        // For now, return empty - full implementation needs infrastructure

        Vec::new()
    }

    /// Get reference count lens
    #[allow(dead_code)]
    fn get_reference_lens(&self, symbol_id: SymbolId, span: Span) -> Vec<CodeLens> {
        // Would need reference tracking to get actual count
        let count = 0; // Placeholder

        vec![CodeLens {
            span,
            kind: CodeLensKind::References,
            title: format!("{} references", count),
            command: Some("cherry.showReferences".to_string()),
            arguments: vec![symbol_id.as_u32().to_string()],
        }]
    }

    /// Get implementation count lens
    #[allow(dead_code)]
    fn get_implementation_lens(&self, symbol_id: SymbolId, span: Span) -> Vec<CodeLens> {
        // Would need to check if method is virtual and find implementations
        let count = 0; // Placeholder

        if count > 0 {
            vec![CodeLens {
                span,
                kind: CodeLensKind::Implementations,
                title: format!("{} implementations", count),
                command: Some("cherry.showImplementations".to_string()),
                arguments: vec![symbol_id.as_u32().to_string()],
            }]
        } else {
            Vec::new()
        }
    }

    /// Get test lens
    #[allow(dead_code)]
    fn get_test_lens(&self, _symbol_id: SymbolId, span: Span) -> Vec<CodeLens> {
        vec![
            CodeLens {
                span,
                kind: CodeLensKind::Test,
                title: "Run test".to_string(),
                command: Some("cherry.runTest".to_string()),
                arguments: Vec::new(),
            },
            CodeLens {
                span,
                kind: CodeLensKind::Test,
                title: "Debug test".to_string(),
                command: Some("cherry.debugTest".to_string()),
                arguments: Vec::new(),
            },
        ]
    }

    /// Get UE5-specific lens
    #[allow(dead_code)]
    fn get_ue5_lens(&self, _symbol_id: SymbolId, span: Span) -> Vec<CodeLens> {
        vec![CodeLens {
            span,
            kind: CodeLensKind::UE5Action,
            title: "Open in Blueprint Editor".to_string(),
            command: Some("cherry.openBlueprint".to_string()),
            arguments: Vec::new(),
        }]
    }

    /// Check if symbol is a test function
    #[allow(dead_code)]
    fn is_test_function(&self, symbol_id: SymbolId) -> bool {
        if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
            let name = self.interner.resolve(symbol.name);
            // Common test naming conventions
            name.starts_with("test_")
                || name.starts_with("Test")
                || name.contains("_test")
                || name.starts_with("TEST")
        } else {
            false
        }
    }

    /// Get code metrics lens
    pub fn get_metrics_lens(&self, symbol_id: SymbolId) -> Option<CodeLens> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;

        // Would calculate complexity, LOC, etc.
        let complexity = 1; // Placeholder

        Some(CodeLens {
            span: symbol.span,
            kind: CodeLensKind::Metrics,
            title: format!("Complexity: {}", complexity),
            command: None,
            arguments: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;

    #[test]
    fn test_code_lens_creation() {
        let file_id = FileId::new(1);
        let lens = CodeLens {
            span: Span::new(file_id, 0, 10),
            kind: CodeLensKind::References,
            title: "5 references".to_string(),
            command: Some("showReferences".to_string()),
            arguments: vec!["123".to_string()],
        };

        assert_eq!(lens.title, "5 references");
        assert_eq!(lens.kind, CodeLensKind::References);
    }

    #[test]
    fn test_code_lens_provider() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let provider = CodeLensProvider::new(&table, &interner);

        let lenses = provider.get_code_lenses(FileId::new(1));
        assert_eq!(lenses.len(), 0); // No symbols
    }

    #[test]
    fn test_is_test_function() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let test_name = interner.intern("test_example");
        let id = table.next_id();
        let symbol = Symbol::new(
            id,
            SymbolKind::Function,
            test_name,
            Span::new(file_id, 0, 10),
            file_id,
        );
        table.add_symbol(symbol);

        let provider = CodeLensProvider::new(&table, &interner);
        assert!(provider.is_test_function(id));
    }
}

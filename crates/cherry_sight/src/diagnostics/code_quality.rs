// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Code quality checks and linting

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
use crate::index::symbol::SymbolKind;
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner};

/// Code quality checker
pub struct CodeQualityChecker<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> CodeQualityChecker<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Check for unused symbols
    pub fn check_unused(&self, _file_id: FileId) -> Vec<Diagnostic> {
        // Would track symbol references and find unused ones
        Vec::new()
    }

    /// Check naming conventions
    pub fn check_naming_conventions(&self, file_id: FileId) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check all symbols in file
        for &symbol_id in &self.symbol_table.symbols_in_file(file_id) {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                let name = self.interner.resolve(symbol.name);

                match symbol.kind {
                    SymbolKind::Class | SymbolKind::UClass => {
                        // Classes should be PascalCase
                        if !name.chars().next().unwrap_or('a').is_uppercase() {
                            diagnostics.push(Diagnostic {
                                severity: DiagnosticSeverity::Warning,
                                span: symbol.span,
                                file_id,
                                message: format!("Class '{}' should start with uppercase", name),
                                code: Some("naming-convention".to_string()),
                                related_information: Vec::new(),
                            });
                        }
                    }
                    SymbolKind::Variable | SymbolKind::Field => {
                        // Variables should be camelCase or snake_case
                        // This is a simplified check
                    }
                    _ => {}
                }
            }
        }

        diagnostics
    }

    /// Check for potential issues
    pub fn check_code_smells(&self, _file_id: FileId) -> Vec<Diagnostic> {
        // Check for:
        // - Functions too long
        // - Too many parameters
        // - Deep nesting
        // - Cyclomatic complexity
        Vec::new()
    }

    /// Run all quality checks
    pub fn check_file(&self, file_id: FileId) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        diagnostics.extend(self.check_unused(file_id));
        diagnostics.extend(self.check_naming_conventions(file_id));
        diagnostics.extend(self.check_code_smells(file_id));

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::Span;

    #[test]
    fn test_naming_convention() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("myClass"); // Bad: should be MyClass
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let checker = CodeQualityChecker::new(&table, &interner);
        let diagnostics = checker.check_naming_conventions(file_id);

        assert!(diagnostics.len() > 0);
        assert!(diagnostics[0].message.contains("should start with uppercase"));
    }
}

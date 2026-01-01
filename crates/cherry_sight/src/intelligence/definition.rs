// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Go to definition and declaration

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Position, Span};

/// Result of a definition lookup
#[derive(Debug, Clone)]
pub struct DefinitionResult {
    pub symbol_id: SymbolId,
    pub span: Span,
    pub file_id: FileId,
    pub kind: DefinitionKind,
}

/// Kind of definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    /// Definition site (where the symbol is defined)
    Definition,
    /// Declaration site (forward declaration, function prototype)
    Declaration,
}

/// Finds definitions and declarations
pub struct DefinitionFinder<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> DefinitionFinder<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Find the definition of a symbol at a given position
    pub fn find_definition(
        &self,
        file_id: FileId,
        position: Position,
    ) -> Option<DefinitionResult> {
        // Find symbol at position
        let symbol_id = self.find_symbol_at_position(file_id, position)?;
        let symbol = self.symbol_table.get_symbol(symbol_id)?;

        // For function declarations, find the definition
        // Note: We would need a forward_declared flag in SymbolFlags for this
        // For now, we'll skip this check
        if symbol.kind == SymbolKind::Function {
            // Look for the actual definition
            if let Some(def_id) = self.find_function_definition(symbol_id) {
                let def_symbol = self.symbol_table.get_symbol(def_id)?;
                return Some(DefinitionResult {
                    symbol_id: def_id,
                    span: def_symbol.span,
                    file_id: def_symbol.file_id,
                    kind: DefinitionKind::Definition,
                });
            }
        }

        // Return the symbol itself as the definition
        Some(DefinitionResult {
            symbol_id,
            span: symbol.span,
            file_id: symbol.file_id,
            kind: DefinitionKind::Definition,
        })
    }

    /// Find the declaration of a symbol
    pub fn find_declaration(
        &self,
        file_id: FileId,
        position: Position,
    ) -> Vec<DefinitionResult> {
        let mut results = Vec::new();

        // Find symbol at position
        let symbol_id = match self.find_symbol_at_position(file_id, position) {
            Some(id) => id,
            None => return results,
        };

        let symbol = match self.symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return results,
        };

        // For methods, find the base class declaration
        if symbol.kind == SymbolKind::Method {
            if let Some(base_decl_id) = self.find_base_method_declaration(symbol_id) {
                if let Some(base_symbol) = self.symbol_table.get_symbol(base_decl_id) {
                    results.push(DefinitionResult {
                        symbol_id: base_decl_id,
                        span: base_symbol.span,
                        file_id: base_symbol.file_id,
                        kind: DefinitionKind::Declaration,
                    });
                }
            }
        }

        // Add the symbol itself
        results.push(DefinitionResult {
            symbol_id,
            span: symbol.span,
            file_id: symbol.file_id,
            kind: DefinitionKind::Definition,
        });

        results
    }

    /// Find symbol at a specific position in a file
    fn find_symbol_at_position(&self, file_id: FileId, position: Position) -> Option<SymbolId> {
        let symbols = self.symbol_table.symbols_in_file(file_id);

        // Convert position to offset (simplified - would need line index in real implementation)
        let offset = position.line * 1000 + position.column; // Rough approximation

        // Find the most specific symbol containing this position
        let mut best_match: Option<(SymbolId, u32)> = None;

        for &symbol_id in &symbols {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                if symbol.span.contains(offset) {
                    let span_size = symbol.span.len();
                    match best_match {
                        None => best_match = Some((symbol_id, span_size)),
                        Some((_, current_size)) if span_size < current_size => {
                            best_match = Some((symbol_id, span_size));
                        }
                        _ => {}
                    }
                }
            }
        }

        best_match.map(|(id, _)| id)
    }

    /// Find the definition of a forward-declared function
    fn find_function_definition(&self, decl_id: SymbolId) -> Option<SymbolId> {
        let decl = self.symbol_table.get_symbol(decl_id)?;

        // Search for a symbol with the same name and parent but without forward declaration flag
        if let Some(parent) = decl.parent {
            let siblings = self.symbol_table.children(parent);
            for &sibling_id in &siblings {
                if sibling_id == decl_id {
                    continue;
                }
                if let Some(sibling) = self.symbol_table.get_symbol(sibling_id) {
                    if sibling.name == decl.name
                        && sibling.kind == SymbolKind::Function {
                        return Some(sibling_id);
                    }
                }
            }
        }

        None
    }

    /// Find the base class method declaration that this method overrides
    fn find_base_method_declaration(&self, method_id: SymbolId) -> Option<SymbolId> {
        let method = self.symbol_table.get_symbol(method_id)?;

        // Check if this method overrides another
        if let Some(overridden_id) = method.overrides {
            return Some(overridden_id);
        }

        // If no direct override link, search base classes for method with same name
        let class_id = method.parent?;
        let bases = self.symbol_table.find_bases(class_id);

        for &base_id in &bases {
            let base_methods = self.symbol_table.children(base_id);
            for &base_method_id in &base_methods {
                if let Some(base_method) = self.symbol_table.get_symbol(base_method_id) {
                    if base_method.kind == SymbolKind::Method && base_method.name == method.name {
                        return Some(base_method_id);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::{Interner, Span};

    #[test]
    fn test_find_symbol_at_position() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");

        // Create a symbol at position 10-20
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 10, 20), file_id);
        table.add_symbol(symbol);

        let finder = DefinitionFinder::new(&table);

        // Position inside the span
        let result = finder.find_symbol_at_position(file_id, Position { line: 0, column: 15 });
        assert!(result.is_some());
    }

    #[test]
    fn test_find_definition() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyFunction");

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Function, name, Span::new(file_id, 10, 20), file_id);
        table.add_symbol(symbol);

        let finder = DefinitionFinder::new(&table);
        let result = finder.find_definition(file_id, Position { line: 0, column: 15 });

        assert!(result.is_some());
        let def = result.unwrap();
        assert_eq!(def.symbol_id, id);
        assert_eq!(def.kind, DefinitionKind::Definition);
    }
}

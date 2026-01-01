// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Hover information provider

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner, Position};

/// Hover contents
#[derive(Debug, Clone)]
pub struct HoverContents {
    pub contents: Vec<String>,
}

/// Provides hover information
pub struct HoverProvider<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> HoverProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get hover information at a position
    pub fn get_hover(&self, _file_id: FileId, _position: Position) -> Option<HoverContents> {
        // Would need to:
        // 1. Find symbol at position
        // 2. Get type information
        // 3. Format documentation

        None
    }

    /// Create hover contents for a symbol
    pub fn create_hover_for_symbol(&self, symbol_id: SymbolId) -> Option<HoverContents> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;
        let name = self.interner.resolve(symbol.name);

        let mut contents = Vec::new();

        // Add symbol signature
        let signature = format!("{} {}", symbol_kind_to_str(symbol.kind), name);
        contents.push(signature);

        // Add documentation if available
        if let Some(ref doc) = symbol.doc_comment {
            contents.push(String::new()); // Blank line
            contents.push(doc.clone());
        }

        Some(HoverContents { contents })
    }
}

fn symbol_kind_to_str(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class | SymbolKind::UClass => "class",
        SymbolKind::Struct | SymbolKind::UStruct => "struct",
        SymbolKind::Enum | SymbolKind::UEnum => "enum",
        SymbolKind::Function | SymbolKind::UFunction => "function",
        SymbolKind::Method => "method",
        SymbolKind::Variable => "variable",
        SymbolKind::Field | SymbolKind::UProperty => "field",
        SymbolKind::Namespace => "namespace",
        _ => "symbol",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::Span;

    #[test]
    fn test_hover_for_symbol() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");
        let id = table.next_id();
        let mut symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        symbol.doc_comment = Some("Test class documentation".to_string());
        table.add_symbol(symbol);

        let provider = HoverProvider::new(&table, &interner);
        let hover = provider.create_hover_for_symbol(id);

        assert!(hover.is_some());
        let contents = hover.unwrap();
        assert!(contents.contents.iter().any(|s| s.contains("class MyClass")));
        assert!(contents.contents.iter().any(|s| s.contains("Test class")));
    }
}

// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol renaming

use crate::index::symbol::SymbolId;
use crate::index::symbol_table::SymbolTable;
use crate::refactoring::{TextEdit, WorkspaceEdit};
use crate::util::{FileId, Interner, Position};

/// Rename operation result
#[derive(Debug, Clone)]
pub struct RenameResult {
    pub old_name: String,
    pub new_name: String,
    pub edits: WorkspaceEdit,
}

/// Provides symbol renaming
pub struct RenameProvider<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> RenameProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Prepare rename - validate and get current name
    pub fn prepare_rename(
        &self,
        _file_id: FileId,
        _position: Position,
    ) -> Option<String> {
        // Would find symbol at position and return its name
        None
    }

    /// Perform rename operation
    pub fn rename(
        &self,
        symbol_id: SymbolId,
        new_name: &str,
    ) -> Option<RenameResult> {
        let symbol = self.symbol_table.get_symbol(symbol_id)?;
        let old_name = self.interner.resolve(symbol.name);

        let mut edits = WorkspaceEdit::new();

        // Rename the definition
        edits.add_edit(TextEdit {
            file_id: symbol.file_id,
            span: symbol.span,
            new_text: new_name.to_string(),
        });

        // Would need to find all references and rename them too
        // This requires reference tracking from AST

        Some(RenameResult {
            old_name,
            new_name: new_name.to_string(),
            edits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::{Symbol, SymbolKind};
    use crate::util::Span;

    #[test]
    fn test_rename() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("oldName");
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Variable, name, Span::new(file_id, 0, 7), file_id);
        table.add_symbol(symbol);

        let provider = RenameProvider::new(&table, &interner);
        let result = provider.rename(id, "newName");

        assert!(result.is_some());
        let rename = result.unwrap();
        assert_eq!(rename.old_name, "oldName");
        assert_eq!(rename.new_name, "newName");
        assert_eq!(rename.edits.changes.len(), 1);
    }
}

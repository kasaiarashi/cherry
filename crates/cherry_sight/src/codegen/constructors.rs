// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Constructor/destructor generation

use crate::index::symbol::{SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::Interner;

/// Generates constructors and destructors
pub struct ConstructorGenerator<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> ConstructorGenerator<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Generate default constructor
    pub fn generate_default_constructor(&self, class_id: SymbolId) -> Option<String> {
        let class = self.symbol_table.get_symbol(class_id)?;
        if !matches!(class.kind, SymbolKind::Class | SymbolKind::UClass) {
            return None;
        }

        let class_name = self.interner.resolve(class.name);
        Some(format!("{}() = default;", class_name))
    }

    /// Generate constructor with member initialization
    pub fn generate_constructor(&self, class_id: SymbolId) -> Option<String> {
        let class = self.symbol_table.get_symbol(class_id)?;
        let class_name = self.interner.resolve(class.name);

        let mut code = format!("{}()", class_name);

        // Would analyze fields and generate initializer list
        // For now, simple version
        code.push_str(" {\n}\n");

        Some(code)
    }

    /// Generate destructor
    pub fn generate_destructor(&self, class_id: SymbolId) -> Option<String> {
        let class = self.symbol_table.get_symbol(class_id)?;
        let class_name = self.interner.resolve(class.name);

        Some(format!("virtual ~{}() = default;", class_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;
    use crate::util::{FileId, Span};

    #[test]
    fn test_generate_constructor() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let generator = ConstructorGenerator::new(&table, &interner);
        let code = generator.generate_default_constructor(id);

        assert!(code.is_some());
        assert!(code.unwrap().contains("MyClass()"));
    }
}

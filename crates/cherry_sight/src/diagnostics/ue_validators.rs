// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5-specific validation

use crate::diagnostics::Diagnostic;
use crate::index::symbol::SymbolId;
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner};

/// UE5 validation rules
pub struct UE5Validator<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> UE5Validator<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Validate UE5 macros in a file
    pub fn validate_macros(&self, _file_id: FileId) -> Vec<Diagnostic> {
        let diagnostics = Vec::new();

        // Would iterate through UCLASS/UPROPERTY/UFUNCTION in file
        // and validate their specifiers
        // For now, placeholder

        diagnostics
    }

    /// Check UCLASS conventions
    pub fn check_uclass_conventions(&self, _class_id: SymbolId) -> Vec<Diagnostic> {
        // Check:
        // - Class name starts with A or U
        // - GENERATED_BODY macro present
        // - Proper constructor signature
        Vec::new()
    }

    /// Check network replication rules
    pub fn check_replication(&self, _file_id: FileId) -> Vec<Diagnostic> {
        // Check:
        // - Replicated properties marked correctly
        // - Server/Client functions have Reliable/Unreliable
        // - Multicast functions proper
        Vec::new()
    }

    /// Run all UE5 validations
    pub fn validate_file(&self, file_id: FileId) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        diagnostics.extend(self.validate_macros(file_id));
        diagnostics.extend(self.check_replication(file_id));

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ue5_validation() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let validator = UE5Validator::new(&table, &interner);

        let diagnostics = validator.validate_file(FileId::new(1));
        assert_eq!(diagnostics.len(), 0);
    }
}

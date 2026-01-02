// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Dead code detection for project-wide analysis

use crate::index::symbol::{Symbol, SymbolId, SymbolKind};
use crate::index::SymbolTable;
use crate::util::{FileId, Interner};
use std::collections::{HashMap, HashSet};

/// Dead code result
#[derive(Debug, Clone)]
pub struct DeadCodeResult {
    pub unused_functions: Vec<SymbolId>,
    pub unused_classes: Vec<SymbolId>,
    pub unused_variables: Vec<SymbolId>,
    pub unreachable_code: Vec<CodeLocation>,
}

/// Code location
#[derive(Debug, Clone)]
pub struct CodeLocation {
    pub file_id: FileId,
    pub line: usize,
    pub column: usize,
}

impl DeadCodeResult {
    pub fn new() -> Self {
        Self {
            unused_functions: Vec::new(),
            unused_classes: Vec::new(),
            unused_variables: Vec::new(),
            unreachable_code: Vec::new(),
        }
    }

    /// Get total count of dead code issues
    pub fn total_issues(&self) -> usize {
        self.unused_functions.len()
            + self.unused_classes.len()
            + self.unused_variables.len()
            + self.unreachable_code.len()
    }

    /// Check if any dead code was found
    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

impl Default for DeadCodeResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Dead code detector
pub struct DeadCodeDetector<'a> {
    #[allow(dead_code)]
    symbol_table: &'a SymbolTable,
    #[allow(dead_code)]
    interner: &'a Interner,
}

impl<'a> DeadCodeDetector<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Detect dead code in the entire project
    pub fn detect_all(&self) -> DeadCodeResult {
        let mut result = DeadCodeResult::new();

        // Build usage graph
        let usage_map = self.build_usage_map();

        // Find unused symbols
        self.find_unused_symbols(&usage_map, &mut result);

        result
    }

    /// Build usage map for all symbols
    fn build_usage_map(&self) -> HashMap<SymbolId, HashSet<SymbolId>> {
        let usage_map: HashMap<SymbolId, HashSet<SymbolId>> = HashMap::new();

        // In full implementation, would analyze:
        // 1. Function calls
        // 2. Class instantiations
        // 3. Variable references
        // 4. Member access
        // For now, return empty map (placeholder)

        usage_map
    }

    /// Find unused symbols
    fn find_unused_symbols(
        &self,
        usage_map: &HashMap<SymbolId, HashSet<SymbolId>>,
        _result: &mut DeadCodeResult,
    ) {
        // In full implementation, would iterate all symbols
        // and check if they're used
        // For now, placeholder implementation

        // Check if symbol has references
        for references in usage_map.values() {
            if references.is_empty() {
                // Symbol is not used
                // Would categorize by symbol kind and add to result
            }
        }
    }

    /// Check if symbol is entry point (should not be marked as dead)
    pub fn is_entry_point(&self, symbol: &Symbol) -> bool {
        match symbol.kind {
            SymbolKind::Function => {
                // UE5 entry points
                let name = self.interner.resolve(symbol.name);
                matches!(
                    name.as_str(),
                    "main"
                        | "BeginPlay"
                        | "Tick"
                        | "EndPlay"
                        | "PostInitializeComponents"
                )
            }
            SymbolKind::Class => {
                // UE5 classes with UCLASS are always kept
                true // Would check for UCLASS macro
            }
            _ => false,
        }
    }

    /// Check if function is Blueprint-callable (should not be marked as dead)
    pub fn is_blueprint_exposed(&self, _symbol: &Symbol) -> bool {
        // Would check for BlueprintCallable, BlueprintPure, etc.
        false
    }

    /// Find unreferenced includes
    pub fn find_unused_includes(&self, _file_id: FileId) -> Vec<String> {
        // Would analyze #include directives and their usage
        Vec::new()
    }
}

/// Dead code statistics
#[derive(Debug, Clone, Copy)]
pub struct DeadCodeStats {
    pub total_symbols: usize,
    pub unused_functions: usize,
    pub unused_classes: usize,
    pub unused_variables: usize,
    pub unreachable_code_count: usize,
}

impl DeadCodeStats {
    pub fn from_result(result: &DeadCodeResult, total_symbols: usize) -> Self {
        Self {
            total_symbols,
            unused_functions: result.unused_functions.len(),
            unused_classes: result.unused_classes.len(),
            unused_variables: result.unused_variables.len(),
            unreachable_code_count: result.unreachable_code.len(),
        }
    }

    /// Get percentage of dead code
    pub fn dead_code_percentage(&self) -> f64 {
        if self.total_symbols == 0 {
            0.0
        } else {
            let dead = self.unused_functions + self.unused_classes + self.unused_variables;
            (dead as f64 / self.total_symbols as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Span;

    #[test]
    fn test_dead_code_result_creation() {
        let result = DeadCodeResult::new();
        assert_eq!(result.total_issues(), 0);
        assert!(!result.has_issues());
    }

    #[test]
    fn test_dead_code_result_with_issues() {
        let mut result = DeadCodeResult::new();
        result.unused_functions.push(SymbolId::new(1));
        result.unused_classes.push(SymbolId::new(2));

        assert_eq!(result.total_issues(), 2);
        assert!(result.has_issues());
    }

    #[test]
    fn test_dead_code_detector_creation() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let detector = DeadCodeDetector::new(&table, &interner);

        assert!(std::ptr::addr_of!(detector) as usize != 0);
    }

    #[test]
    fn test_is_entry_point() {
        let table = SymbolTable::new();
        let mut interner = Interner::new();
        let detector = DeadCodeDetector::new(&table, &interner);

        let file_id = FileId::new(1);
        let main_name = interner.intern("main");
        let main_symbol = Symbol::new(
            SymbolId::new(1),
            SymbolKind::Function,
            main_name,
            Span::new(file_id, 0, 10),
            file_id,
        );

        assert!(detector.is_entry_point(&main_symbol));
    }

    #[test]
    fn test_detect_all() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let detector = DeadCodeDetector::new(&table, &interner);

        let result = detector.detect_all();
        assert_eq!(result.total_issues(), 0);
    }

    #[test]
    fn test_dead_code_stats() {
        let result = DeadCodeResult::new();
        let stats = DeadCodeStats::from_result(&result, 100);

        assert_eq!(stats.total_symbols, 100);
        assert_eq!(stats.dead_code_percentage(), 0.0);
    }

    #[test]
    fn test_dead_code_percentage() {
        let mut result = DeadCodeResult::new();
        result.unused_functions.push(SymbolId::new(1));
        result.unused_functions.push(SymbolId::new(2));

        let stats = DeadCodeStats::from_result(&result, 10);
        assert_eq!(stats.dead_code_percentage(), 20.0);
    }
}

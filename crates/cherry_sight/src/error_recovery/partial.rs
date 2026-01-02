// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Partial analysis capabilities for degraded code

use crate::index::symbol::SymbolId;
use crate::index::SymbolTable;
use crate::util::{FileId, Interner};
use std::collections::HashSet;

/// Partial analysis result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStatus {
    /// Analysis completed successfully
    Complete,
    /// Analysis completed with some errors
    Partial,
    /// Analysis failed but some results available
    Degraded,
    /// Analysis failed completely
    Failed,
}

/// Partial analysis result
#[derive(Debug, Clone)]
pub struct PartialAnalysisResult {
    pub status: AnalysisStatus,
    pub symbols_found: usize,
    pub errors_encountered: usize,
    pub warnings: Vec<String>,
    pub incomplete_regions: Vec<(usize, usize)>, // (start, end) byte offsets
}

impl PartialAnalysisResult {
    pub fn new(status: AnalysisStatus) -> Self {
        Self {
            status,
            symbols_found: 0,
            errors_encountered: 0,
            warnings: Vec::new(),
            incomplete_regions: Vec::new(),
        }
    }

    /// Check if analysis has usable results
    pub fn has_results(&self) -> bool {
        matches!(
            self.status,
            AnalysisStatus::Complete | AnalysisStatus::Partial | AnalysisStatus::Degraded
        )
    }

    /// Add warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Mark region as incomplete
    pub fn mark_incomplete(&mut self, start: usize, end: usize) {
        self.incomplete_regions.push((start, end));
    }
}

/// Partial analyzer for handling incomplete code
pub struct PartialAnalyzer<'a> {
    #[allow(dead_code)]
    symbol_table: &'a SymbolTable,
    #[allow(dead_code)]
    interner: &'a Interner,
}

impl<'a> PartialAnalyzer<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Analyze file even with errors
    pub fn analyze_partial(&self, _file_id: FileId) -> PartialAnalysisResult {
        let mut result = PartialAnalysisResult::new(AnalysisStatus::Partial);

        // Count symbols we could successfully index
        // In full implementation, would iterate file symbols
        // For now, count all symbols (placeholder)
        result.symbols_found = 0; // Would use: self.count_file_symbols(file_id);

        if result.symbols_found > 0 {
            result.status = AnalysisStatus::Partial;
        } else {
            result.status = AnalysisStatus::Failed;
        }

        result
    }

    /// Get symbols that were successfully analyzed
    pub fn get_valid_symbols(&self, _file_id: FileId) -> Vec<SymbolId> {
        // In full implementation, would filter symbols by validation
        // For now, return empty (placeholder)
        Vec::new()
    }

    /// Check if a symbol is fully analyzed
    pub fn is_symbol_complete(&self, _symbol_id: SymbolId) -> bool {
        // In full implementation, would check symbol completeness
        // For now, return true (placeholder)
        true
    }

    /// Get symbols with incomplete analysis
    pub fn get_incomplete_symbols(&self, _file_id: FileId) -> Vec<SymbolId> {
        // In full implementation, would track incomplete symbols
        Vec::new()
    }
}

/// Analysis completeness tracker
pub struct CompletenessTracker {
    complete_files: HashSet<FileId>,
    partial_files: HashSet<FileId>,
    failed_files: HashSet<FileId>,
}

impl CompletenessTracker {
    pub fn new() -> Self {
        Self {
            complete_files: HashSet::new(),
            partial_files: HashSet::new(),
            failed_files: HashSet::new(),
        }
    }

    /// Mark file as completely analyzed
    pub fn mark_complete(&mut self, file_id: FileId) {
        self.complete_files.insert(file_id);
        self.partial_files.remove(&file_id);
        self.failed_files.remove(&file_id);
    }

    /// Mark file as partially analyzed
    pub fn mark_partial(&mut self, file_id: FileId) {
        self.partial_files.insert(file_id);
        self.complete_files.remove(&file_id);
        self.failed_files.remove(&file_id);
    }

    /// Mark file as failed
    pub fn mark_failed(&mut self, file_id: FileId) {
        self.failed_files.insert(file_id);
        self.complete_files.remove(&file_id);
        self.partial_files.remove(&file_id);
    }

    /// Get completeness status
    pub fn get_status(&self, file_id: FileId) -> AnalysisStatus {
        if self.complete_files.contains(&file_id) {
            AnalysisStatus::Complete
        } else if self.partial_files.contains(&file_id) {
            AnalysisStatus::Partial
        } else {
            AnalysisStatus::Failed
        }
    }

    /// Get statistics
    pub fn stats(&self) -> CompletenessStats {
        CompletenessStats {
            complete: self.complete_files.len(),
            partial: self.partial_files.len(),
            failed: self.failed_files.len(),
        }
    }

    /// Clear tracking for file
    pub fn clear(&mut self, file_id: FileId) {
        self.complete_files.remove(&file_id);
        self.partial_files.remove(&file_id);
        self.failed_files.remove(&file_id);
    }
}

impl Default for CompletenessTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Completeness statistics
#[derive(Debug, Clone, Copy)]
pub struct CompletenessStats {
    pub complete: usize,
    pub partial: usize,
    pub failed: usize,
}

impl CompletenessStats {
    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        let total = self.complete + self.partial + self.failed;
        if total == 0 {
            0.0
        } else {
            (self.complete as f64 / total as f64) * 100.0
        }
    }

    /// Get total files
    pub fn total(&self) -> usize {
        self.complete + self.partial + self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_status() {
        let result = PartialAnalysisResult::new(AnalysisStatus::Complete);
        assert_eq!(result.status, AnalysisStatus::Complete);
        assert!(result.has_results());
    }

    #[test]
    fn test_partial_result_warnings() {
        let mut result = PartialAnalysisResult::new(AnalysisStatus::Partial);
        result.add_warning("Missing semicolon".to_string());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_incomplete_regions() {
        let mut result = PartialAnalysisResult::new(AnalysisStatus::Degraded);
        result.mark_incomplete(10, 20);
        result.mark_incomplete(50, 60);
        assert_eq!(result.incomplete_regions.len(), 2);
    }

    #[test]
    fn test_partial_analyzer() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let analyzer = PartialAnalyzer::new(&table, &interner);

        let result = analyzer.analyze_partial(FileId::new(1));
        assert!(matches!(
            result.status,
            AnalysisStatus::Partial | AnalysisStatus::Failed
        ));
    }

    #[test]
    fn test_completeness_tracker() {
        let mut tracker = CompletenessTracker::new();
        let file_id = FileId::new(1);

        tracker.mark_complete(file_id);
        assert_eq!(tracker.get_status(file_id), AnalysisStatus::Complete);

        tracker.mark_partial(file_id);
        assert_eq!(tracker.get_status(file_id), AnalysisStatus::Partial);

        tracker.mark_failed(file_id);
        assert_eq!(tracker.get_status(file_id), AnalysisStatus::Failed);
    }

    #[test]
    fn test_completeness_stats() {
        let mut tracker = CompletenessTracker::new();

        tracker.mark_complete(FileId::new(1));
        tracker.mark_complete(FileId::new(2));
        tracker.mark_partial(FileId::new(3));
        tracker.mark_failed(FileId::new(4));

        let stats = tracker.stats();
        assert_eq!(stats.complete, 2);
        assert_eq!(stats.partial, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.total(), 4);
        assert_eq!(stats.completion_percentage(), 50.0);
    }
}

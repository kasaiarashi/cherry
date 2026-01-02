// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Code complexity metrics

use crate::index::symbol::SymbolId;
use crate::util::FileId;
use std::collections::HashMap;

/// Cyclomatic complexity result
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub lines_of_code: usize,
    pub nesting_depth: usize,
}

impl ComplexityMetrics {
    pub fn new() -> Self {
        Self {
            cyclomatic_complexity: 1, // Minimum is 1
            cognitive_complexity: 0,
            lines_of_code: 0,
            nesting_depth: 0,
        }
    }

    /// Check if complexity is high
    pub fn is_high_complexity(&self) -> bool {
        self.cyclomatic_complexity > 10 || self.cognitive_complexity > 15
    }

    /// Get complexity rating
    pub fn get_rating(&self) -> ComplexityRating {
        if self.cyclomatic_complexity <= 5 {
            ComplexityRating::Low
        } else if self.cyclomatic_complexity <= 10 {
            ComplexityRating::Medium
        } else if self.cyclomatic_complexity <= 20 {
            ComplexityRating::High
        } else {
            ComplexityRating::VeryHigh
        }
    }
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Complexity rating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityRating {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Complexity analyzer
pub struct ComplexityAnalyzer {
    // Analyzer state
}

impl ComplexityAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Calculate cyclomatic complexity for a function
    pub fn calculate_cyclomatic(&self, _symbol_id: SymbolId) -> usize {
        // In full implementation, would analyze AST:
        // 1. Count decision points (if, while, for, case, &&, ||, etc.)
        // 2. Calculate edges and nodes in control flow graph
        // For now, return placeholder
        1
    }

    /// Calculate cognitive complexity
    pub fn calculate_cognitive(&self, _symbol_id: SymbolId) -> usize {
        // In full implementation, would analyze:
        // 1. Nesting depth (higher weight for deeper nesting)
        // 2. Control flow breaks (break, continue, return)
        // 3. Recursion
        // For now, return placeholder
        0
    }

    /// Calculate all metrics for a function
    pub fn analyze_function(&self, symbol_id: SymbolId) -> ComplexityMetrics {
        ComplexityMetrics {
            cyclomatic_complexity: self.calculate_cyclomatic(symbol_id),
            cognitive_complexity: self.calculate_cognitive(symbol_id),
            lines_of_code: self.count_lines(symbol_id),
            nesting_depth: self.calculate_nesting_depth(symbol_id),
        }
    }

    /// Count lines of code
    fn count_lines(&self, _symbol_id: SymbolId) -> usize {
        // Would count actual source lines
        0
    }

    /// Calculate maximum nesting depth
    fn calculate_nesting_depth(&self, _symbol_id: SymbolId) -> usize {
        // Would analyze AST for maximum nesting depth
        0
    }

    /// Find functions with high complexity
    pub fn find_high_complexity_functions(
        &self,
        _symbols: &[SymbolId],
    ) -> Vec<(SymbolId, ComplexityMetrics)> {
        // Would analyze all functions and filter high complexity ones
        Vec::new()
    }
}

impl Default for ComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Code maintainability index
pub struct MaintainabilityIndex {
    // Maintainability calculator
}

impl MaintainabilityIndex {
    pub fn new() -> Self {
        Self {}
    }

    /// Calculate maintainability index (0-100)
    /// Based on Halstead Volume, Cyclomatic Complexity, and Lines of Code
    pub fn calculate(&self, metrics: &ComplexityMetrics) -> f64 {
        // Simplified maintainability index calculation
        // MI = 171 - 5.2 * ln(HV) - 0.23 * CC - 16.2 * ln(LOC)
        // For now, simple heuristic

        let loc = metrics.lines_of_code.max(1) as f64;
        let cc = metrics.cyclomatic_complexity as f64;

        let mi = 100.0 - (cc * 2.0) - (loc / 10.0);
        mi.clamp(0.0, 100.0)
    }

    /// Get maintainability rating
    pub fn get_rating(&self, index: f64) -> MaintainabilityRating {
        if index >= 85.0 {
            MaintainabilityRating::Excellent
        } else if index >= 65.0 {
            MaintainabilityRating::Good
        } else if index >= 50.0 {
            MaintainabilityRating::Fair
        } else {
            MaintainabilityRating::Poor
        }
    }
}

impl Default for MaintainabilityIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Maintainability rating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintainabilityRating {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// Project-wide complexity summary
#[derive(Debug, Clone)]
pub struct ProjectComplexityReport {
    pub file_metrics: HashMap<FileId, FileComplexityMetrics>,
    pub total_functions: usize,
    pub high_complexity_functions: usize,
}

/// File complexity metrics
#[derive(Debug, Clone)]
pub struct FileComplexityMetrics {
    pub file_id: FileId,
    pub average_complexity: f64,
    pub max_complexity: usize,
    pub function_count: usize,
}

impl ProjectComplexityReport {
    pub fn new() -> Self {
        Self {
            file_metrics: HashMap::new(),
            total_functions: 0,
            high_complexity_functions: 0,
        }
    }

    /// Get average complexity across project
    pub fn average_complexity(&self) -> f64 {
        if self.file_metrics.is_empty() {
            return 0.0;
        }

        let sum: f64 = self
            .file_metrics
            .values()
            .map(|m| m.average_complexity)
            .sum();
        sum / self.file_metrics.len() as f64
    }

    /// Get percentage of high complexity functions
    pub fn high_complexity_percentage(&self) -> f64 {
        if self.total_functions == 0 {
            return 0.0;
        }
        (self.high_complexity_functions as f64 / self.total_functions as f64) * 100.0
    }
}

impl Default for ProjectComplexityReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_metrics_creation() {
        let metrics = ComplexityMetrics::new();
        assert_eq!(metrics.cyclomatic_complexity, 1);
        assert!(!metrics.is_high_complexity());
    }

    #[test]
    fn test_high_complexity_detection() {
        let high_metrics = ComplexityMetrics {
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            lines_of_code: 200,
            nesting_depth: 5,
        };

        assert!(high_metrics.is_high_complexity());
    }

    #[test]
    fn test_complexity_rating() {
        let low = ComplexityMetrics {
            cyclomatic_complexity: 3,
            ..Default::default()
        };
        assert_eq!(low.get_rating(), ComplexityRating::Low);

        let high = ComplexityMetrics {
            cyclomatic_complexity: 15,
            ..Default::default()
        };
        assert_eq!(high.get_rating(), ComplexityRating::High);
    }

    #[test]
    fn test_complexity_analyzer() {
        let analyzer = ComplexityAnalyzer::new();
        let symbol_id = SymbolId::new(1);

        let complexity = analyzer.calculate_cyclomatic(symbol_id);
        assert_eq!(complexity, 1); // Minimum complexity
    }

    #[test]
    fn test_analyze_function() {
        let analyzer = ComplexityAnalyzer::new();
        let metrics = analyzer.analyze_function(SymbolId::new(1));

        assert!(metrics.cyclomatic_complexity >= 1);
    }

    #[test]
    fn test_maintainability_index() {
        let mi = MaintainabilityIndex::new();
        let metrics = ComplexityMetrics {
            cyclomatic_complexity: 5,
            cognitive_complexity: 3,
            lines_of_code: 50,
            nesting_depth: 2,
        };

        let index = mi.calculate(&metrics);
        assert!(index >= 0.0 && index <= 100.0);
    }

    #[test]
    fn test_maintainability_rating() {
        let mi = MaintainabilityIndex::new();

        assert_eq!(mi.get_rating(90.0), MaintainabilityRating::Excellent);
        assert_eq!(mi.get_rating(70.0), MaintainabilityRating::Good);
        assert_eq!(mi.get_rating(55.0), MaintainabilityRating::Fair);
        assert_eq!(mi.get_rating(30.0), MaintainabilityRating::Poor);
    }

    #[test]
    fn test_project_complexity_report() {
        let report = ProjectComplexityReport::new();
        assert_eq!(report.average_complexity(), 0.0);
        assert_eq!(report.high_complexity_percentage(), 0.0);
    }

    #[test]
    fn test_project_stats() {
        let mut report = ProjectComplexityReport::new();
        report.total_functions = 100;
        report.high_complexity_functions = 20;

        assert_eq!(report.high_complexity_percentage(), 20.0);
    }
}

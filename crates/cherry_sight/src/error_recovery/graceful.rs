// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Graceful error handling for robust code analysis

use crate::ast::ParseError;
use crate::util::FileId;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Error severity for graceful degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational message
    Info,
    /// Warning - analysis can continue
    Warning,
    /// Error - analysis degraded but can continue
    Error,
    /// Fatal error - analysis cannot continue
    Fatal,
}

/// Recoverable error with context
#[derive(Debug, Clone)]
pub struct RecoverableError {
    pub severity: ErrorSeverity,
    pub message: String,
    pub file_id: FileId,
    pub location: Option<(usize, usize)>, // (line, column)
    pub context: String,
}

impl RecoverableError {
    pub fn new(
        severity: ErrorSeverity,
        message: String,
        file_id: FileId,
        location: Option<(usize, usize)>,
        context: String,
    ) -> Self {
        Self {
            severity,
            message,
            file_id,
            location,
            context,
        }
    }

    /// Create error from parse error
    pub fn from_parse_error(error: &ParseError, file_id: FileId) -> Self {
        Self {
            severity: ErrorSeverity::Error,
            message: error.message.clone(),
            file_id,
            location: None,
            context: "Parse error".to_string(),
        }
    }

    /// Check if error is fatal
    pub fn is_fatal(&self) -> bool {
        self.severity == ErrorSeverity::Fatal
    }

    /// Check if error allows continued analysis
    pub fn allows_continuation(&self) -> bool {
        self.severity < ErrorSeverity::Fatal
    }
}

/// Error recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Skip the problematic section
    Skip,
    /// Use best-guess heuristics
    Heuristic,
    /// Use partial results
    Partial,
    /// Abort analysis
    Abort,
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    errors: Arc<RwLock<HashMap<FileId, Vec<RecoverableError>>>>,
    max_errors_per_file: usize,
    default_strategy: RecoveryStrategy,
}

impl ErrorRecoveryManager {
    pub fn new(max_errors_per_file: usize, default_strategy: RecoveryStrategy) -> Self {
        Self {
            errors: Arc::new(RwLock::new(HashMap::new())),
            max_errors_per_file,
            default_strategy,
        }
    }

    /// Record an error
    pub fn record_error(&self, error: RecoverableError) -> bool {
        let mut errors = self.errors.write().unwrap();
        let file_errors = errors.entry(error.file_id).or_default();

        file_errors.push(error.clone());

        // Check if we've exceeded max errors
        if file_errors.len() >= self.max_errors_per_file {
            return false; // Cannot continue
        }

        error.allows_continuation()
    }

    /// Get errors for a file
    pub fn get_errors(&self, file_id: FileId) -> Vec<RecoverableError> {
        self.errors
            .read()
            .unwrap()
            .get(&file_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear errors for a file
    pub fn clear_errors(&self, file_id: FileId) {
        self.errors.write().unwrap().remove(&file_id);
    }

    /// Get total error count
    pub fn total_errors(&self) -> usize {
        self.errors
            .read()
            .unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Determine recovery strategy for error
    pub fn get_strategy(&self, error: &RecoverableError) -> RecoveryStrategy {
        match error.severity {
            ErrorSeverity::Info | ErrorSeverity::Warning => RecoveryStrategy::Partial,
            ErrorSeverity::Error => self.default_strategy,
            ErrorSeverity::Fatal => RecoveryStrategy::Abort,
        }
    }

    /// Check if analysis should continue
    pub fn should_continue(&self, file_id: FileId) -> bool {
        let errors = self.get_errors(file_id);

        // Don't continue if we have fatal errors
        if errors.iter().any(|e| e.is_fatal()) {
            return false;
        }

        // Don't continue if we've exceeded max errors
        errors.len() < self.max_errors_per_file
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new(100, RecoveryStrategy::Partial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Fatal);
    }

    #[test]
    fn test_recoverable_error_creation() {
        let error = RecoverableError::new(
            ErrorSeverity::Error,
            "Test error".to_string(),
            FileId::new(1),
            Some((10, 5)),
            "Test context".to_string(),
        );

        assert_eq!(error.severity, ErrorSeverity::Error);
        assert_eq!(error.message, "Test error");
        assert!(!error.is_fatal());
        assert!(error.allows_continuation());
    }

    #[test]
    fn test_error_recovery_manager() {
        let manager = ErrorRecoveryManager::new(10, RecoveryStrategy::Partial);
        let file_id = FileId::new(1);

        let error = RecoverableError::new(
            ErrorSeverity::Warning,
            "Warning".to_string(),
            file_id,
            None,
            "Context".to_string(),
        );

        assert!(manager.record_error(error));
        assert_eq!(manager.get_errors(file_id).len(), 1);
        assert_eq!(manager.total_errors(), 1);
    }

    #[test]
    fn test_max_errors_limit() {
        let manager = ErrorRecoveryManager::new(3, RecoveryStrategy::Partial);
        let file_id = FileId::new(1);

        for i in 0..3 {
            let error = RecoverableError::new(
                ErrorSeverity::Error,
                format!("Error {}", i),
                file_id,
                None,
                "Context".to_string(),
            );
            manager.record_error(error);
        }

        // Should stop after max errors
        let error = RecoverableError::new(
            ErrorSeverity::Error,
            "Too many errors".to_string(),
            file_id,
            None,
            "Context".to_string(),
        );
        assert!(!manager.record_error(error));
    }

    #[test]
    fn test_recovery_strategy() {
        let manager = ErrorRecoveryManager::default();

        let info = RecoverableError::new(
            ErrorSeverity::Info,
            "Info".to_string(),
            FileId::new(1),
            None,
            "Context".to_string(),
        );
        assert_eq!(manager.get_strategy(&info), RecoveryStrategy::Partial);

        let fatal = RecoverableError::new(
            ErrorSeverity::Fatal,
            "Fatal".to_string(),
            FileId::new(1),
            None,
            "Context".to_string(),
        );
        assert_eq!(manager.get_strategy(&fatal), RecoveryStrategy::Abort);
    }
}

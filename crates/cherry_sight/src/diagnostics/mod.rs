// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Real-time diagnostics and error detection
//!
//! This module provides:
//! - Syntax error highlighting
//! - Semantic error detection
//! - UE5-specific warnings
//! - Code quality checks

pub mod syntax;
pub mod semantic;
pub mod ue_validators;
pub mod code_quality;

pub use syntax::SyntaxDiagnostics;
pub use semantic::SemanticDiagnostics;
pub use ue_validators::UE5Validator;
pub use code_quality::CodeQualityChecker;

use crate::util::{FileId, Span};

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub span: Span,
    pub file_id: FileId,
    pub message: String,
    pub code: Option<String>,
    pub related_information: Vec<RelatedInformation>,
}

/// Related diagnostic information
#[derive(Debug, Clone)]
pub struct RelatedInformation {
    pub span: Span,
    pub file_id: FileId,
    pub message: String,
}

impl Diagnostic {
    pub fn error(file_id: FileId, span: Span, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            span,
            file_id,
            message,
            code: None,
            related_information: Vec::new(),
        }
    }

    pub fn warning(file_id: FileId, span: Span, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            span,
            file_id,
            message,
            code: None,
            related_information: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Syntax error diagnostics

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
use crate::ast::ParseError;
use crate::util::FileId;

/// Syntax diagnostics provider
pub struct SyntaxDiagnostics;

impl SyntaxDiagnostics {
    pub fn new() -> Self {
        Self
    }

    /// Convert parse errors to diagnostics
    pub fn from_parse_errors(
        file_id: FileId,
        errors: &[ParseError],
    ) -> Vec<Diagnostic> {
        errors
            .iter()
            .map(|err| Diagnostic {
                severity: DiagnosticSeverity::Error,
                span: err.span,
                file_id,
                message: err.message.clone(),
                code: Some("syntax-error".to_string()),
                related_information: Vec::new(),
            })
            .collect()
    }

    /// Check for common syntax issues
    pub fn check_syntax(&self, _file_id: FileId) -> Vec<Diagnostic> {
        // Would analyze AST for syntax patterns
        // - Missing semicolons
        // - Unmatched braces
        // - Invalid declarations
        Vec::new()
    }
}

impl Default for SyntaxDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Span;

    #[test]
    fn test_parse_error_conversion() {
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);
        let errors = vec![ParseError {
            span,
            message: "Expected semicolon".to_string(),
        }];

        let diagnostics = SyntaxDiagnostics::from_parse_errors(file_id, &errors);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostics[0].message, "Expected semicolon");
    }
}

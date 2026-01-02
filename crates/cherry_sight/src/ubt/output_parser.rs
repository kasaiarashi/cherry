// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UBT output parsing

use std::path::PathBuf;
use regex::Regex;

/// Compiler error from UBT output
#[derive(Debug, Clone)]
pub struct CompileError {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub error_code: Option<String>,
}

/// Compiler warning from UBT output
#[derive(Debug, Clone)]
pub struct CompileWarning {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub warning_code: Option<String>,
}

/// Build statistics
#[derive(Debug, Clone, Default)]
pub struct BuildStatistics {
    pub total_files: usize,
    pub compiled_files: usize,
    pub errors: usize,
    pub warnings: usize,
    pub duration_seconds: f64,
}

/// Parses UBT output
pub struct UBTOutputParser {
    error_regex: Regex,
    warning_regex: Regex,
    file_regex: Regex,
}

impl UBTOutputParser {
    pub fn new() -> Self {
        Self {
            // MSVC format: file(line,col): error C1234: message
            error_regex: Regex::new(
                r"(?P<file>[^(]+)\((?P<line>\d+),(?P<col>\d+)\):\s*error\s+(?P<code>\w+):\s*(?P<msg>.*)"
            ).unwrap(),
            warning_regex: Regex::new(
                r"(?P<file>[^(]+)\((?P<line>\d+),(?P<col>\d+)\):\s*warning\s+(?P<code>\w+):\s*(?P<msg>.*)"
            ).unwrap(),
            file_regex: Regex::new(
                r"^\s*(?:Compiling|Parsing)\s+(?P<file>.*\.(?:cpp|h))"
            ).unwrap(),
        }
    }

    /// Parse build output
    pub fn parse(&self, output: &str) -> ParsedOutput {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut compiled_files = Vec::new();
        let mut stats = BuildStatistics::default();

        for line in output.lines() {
            // Parse errors
            if let Some(error) = self.parse_error(line) {
                errors.push(error);
                stats.errors += 1;
            }

            // Parse warnings
            if let Some(warning) = self.parse_warning(line) {
                warnings.push(warning);
                stats.warnings += 1;
            }

            // Parse compiled files
            if let Some(file) = self.parse_compiled_file(line) {
                compiled_files.push(file);
                stats.compiled_files += 1;
            }

            // Parse build statistics
            if line.contains("Total build time:") {
                if let Some(duration) = self.parse_build_time(line) {
                    stats.duration_seconds = duration;
                }
            }
        }

        ParsedOutput {
            errors,
            warnings,
            compiled_files,
            statistics: stats,
        }
    }

    /// Parse a single error line
    fn parse_error(&self, line: &str) -> Option<CompileError> {
        let caps = self.error_regex.captures(line)?;

        Some(CompileError {
            file: PathBuf::from(&caps["file"]),
            line: caps["line"].parse().ok()?,
            column: caps["col"].parse().ok()?,
            message: caps["msg"].to_string(),
            error_code: Some(caps["code"].to_string()),
        })
    }

    /// Parse a single warning line
    fn parse_warning(&self, line: &str) -> Option<CompileWarning> {
        let caps = self.warning_regex.captures(line)?;

        Some(CompileWarning {
            file: PathBuf::from(&caps["file"]),
            line: caps["line"].parse().ok()?,
            column: caps["col"].parse().ok()?,
            message: caps["msg"].to_string(),
            warning_code: Some(caps["code"].to_string()),
        })
    }

    /// Parse compiled file line
    fn parse_compiled_file(&self, line: &str) -> Option<PathBuf> {
        let caps = self.file_regex.captures(line)?;
        Some(PathBuf::from(&caps["file"]))
    }

    /// Parse build time
    fn parse_build_time(&self, line: &str) -> Option<f64> {
        // Format: "Total build time: 123.45 seconds"
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "time:" && i + 1 < parts.len() {
                return parts[i + 1].parse().ok();
            }
        }
        None
    }

    /// Check if build was successful
    pub fn is_success(&self, output: &str) -> bool {
        output.contains("BUILD SUCCESSFUL") ||
        (output.contains("Completed") && !output.contains("error"))
    }

    /// Get error summary
    pub fn get_error_summary(&self, errors: &[CompileError]) -> String {
        if errors.is_empty() {
            return "No errors".to_string();
        }

        format!(
            "{} error(s) found:\n{}",
            errors.len(),
            errors
                .iter()
                .take(5)
                .map(|e| format!("  {}({}): {}", e.file.display(), e.line, e.message))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

/// Parsed build output
#[derive(Debug, Clone)]
pub struct ParsedOutput {
    pub errors: Vec<CompileError>,
    pub warnings: Vec<CompileWarning>,
    pub compiled_files: Vec<PathBuf>,
    pub statistics: BuildStatistics,
}

impl Default for UBTOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error() {
        let parser = UBTOutputParser::new();
        let line = r"C:\Project\Source\MyClass.cpp(42,10): error C2065: undeclared identifier";

        let error = parser.parse_error(line);
        assert!(error.is_some());

        let err = error.unwrap();
        assert_eq!(err.line, 42);
        assert_eq!(err.column, 10);
        assert!(err.file.to_string_lossy().contains("MyClass.cpp"));
    }

    #[test]
    fn test_parse_warning() {
        let parser = UBTOutputParser::new();
        let line = r"C:\Project\Source\MyClass.cpp(100,5): warning C4101: unreferenced local variable";

        let warning = parser.parse_warning(line);
        assert!(warning.is_some());

        let warn = warning.unwrap();
        assert_eq!(warn.line, 100);
        assert_eq!(warn.column, 5);
    }

    #[test]
    fn test_parse_compiled_file() {
        let parser = UBTOutputParser::new();
        let line = "  Compiling MyClass.cpp";

        let file = parser.parse_compiled_file(line);
        assert!(file.is_some());
    }

    #[test]
    fn test_parse_build_output() {
        let parser = UBTOutputParser::new();
        let output = r"Compiling MyClass.cpp
C:\Project\Source\MyClass.cpp(42,10): error C2065: undeclared identifier
C:\Project\Source\MyClass.cpp(100,5): warning C4101: unreferenced local variable
Total build time: 45.67 seconds";

        let parsed = parser.parse(output);

        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.warnings.len(), 1);
        assert_eq!(parsed.compiled_files.len(), 1);
        assert!((parsed.statistics.duration_seconds - 45.67).abs() < 0.01);
    }

    #[test]
    fn test_is_success() {
        let parser = UBTOutputParser::new();

        assert!(parser.is_success("BUILD SUCCESSFUL"));
        assert!(parser.is_success("Completed in 30 seconds"));
        assert!(!parser.is_success("error C2065: undeclared"));
    }
}

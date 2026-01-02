// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Database for managing parsed files and queries
//!
//! Note: This is a simplified implementation for Phase 1.
//! Salsa will be integrated in Phase 2 for incremental computation.

pub mod inputs;
pub mod derived;

pub use inputs::SourceFile;
pub use derived::{parse, syntax_errors, line_index, LineIndex};

use crate::util::{FileId, Interner};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// The main database for cherry-sight
#[derive(Clone, Debug)]
pub struct Database {
    interner: Arc<RwLock<Interner>>,
    source_files: Arc<RwLock<HashMap<FileId, SourceFile>>>,
}

impl Database {
    /// Create a new database
    pub fn new() -> Self {
        Self {
            interner: Arc::new(RwLock::new(Interner::new())),
            source_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a source file to the database
    pub fn add_source_file(&self, file_id: FileId, path: PathBuf, content: Arc<String>) -> SourceFile {
        let source = SourceFile::new(file_id, path, content);
        self.source_files.write().insert(file_id, source.clone());
        source
    }

    /// Get a source file by ID
    pub fn get_source_file(&self, file_id: FileId) -> Option<SourceFile> {
        self.source_files.read().get(&file_id).cloned()
    }

    /// Remove a source file from the database
    pub fn remove_source_file(&self, file_id: FileId) {
        self.source_files.write().remove(&file_id);
    }

    /// Update the content of an existing source file
    pub fn update_source_content(&self, file_id: FileId, new_content: Arc<String>) {
        if let Some(source) = self.source_files.read().get(&file_id) {
            let updated = SourceFile::new(file_id, source.path.clone(), new_content);
            self.source_files.write().insert(file_id, updated);
        }
    }

    /// Get all source files
    pub fn all_source_files(&self) -> Vec<SourceFile> {
        self.source_files.read().values().cloned().collect()
    }

    /// Get the string interner
    pub fn interner(&self) -> Arc<RwLock<Interner>> {
        self.interner.clone()
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new();
        assert_eq!(db.all_source_files().len(), 0);
    }

    #[test]
    fn test_add_source_file() {
        let db = Database::new();

        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        let content = Arc::new("int main() {}".to_string());

        db.add_source_file(file_id, path, content);

        assert_eq!(db.all_source_files().len(), 1);
        assert!(db.get_source_file(file_id).is_some());
    }

    #[test]
    fn test_remove_source_file() {
        let db = Database::new();

        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        let content = Arc::new("int main() {}".to_string());

        db.add_source_file(file_id, path, content);
        assert_eq!(db.all_source_files().len(), 1);

        db.remove_source_file(file_id);
        assert_eq!(db.all_source_files().len(), 0);
    }

    #[test]
    fn test_parse_query() {
        let db = Database::new();

        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        let content = Arc::new("int main() { return 0; }".to_string());

        let source = db.add_source_file(file_id, path, content);

        // Test that parse query works
        let unit = parse(&source);
        assert_eq!(unit.file_id, file_id);
        assert_eq!(unit.declarations.len(), 1);
    }

    #[test]
    fn test_line_index_query() {
        let db = Database::new();

        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        let content = Arc::new("line 1\nline 2\nline 3".to_string());

        let source = db.add_source_file(file_id, path, content);

        let index = line_index(&source);
        assert_eq!(index.line(0), 0);
        assert_eq!(index.line(7), 1);
        assert_eq!(index.line(14), 2);
    }

    #[test]
    fn test_syntax_errors_query() {
        let db = Database::new();

        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        // Missing semicolon
        let content = Arc::new("int main() { return 0 }".to_string());

        let source = db.add_source_file(file_id, path, content);

        let errors = syntax_errors(&source);
        assert!(errors.len() > 0, "Should have parse errors");
    }

    #[test]
    fn test_interner_access() {
        let db = Database::new();

        let interner = db.interner();
        let text = "test_string";

        let interned = interner.write().intern(text);
        let resolved = interner.read().resolve(interned);

        assert_eq!(resolved, text);
    }
}

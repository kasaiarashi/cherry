// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Input data structures for file content and metadata

use crate::util::FileId;
use std::path::PathBuf;
use std::sync::Arc;

/// Source file input
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The unique file identifier
    pub file_id: FileId,

    /// The absolute path to the source file
    pub path: PathBuf,

    /// The source code content
    pub content: Arc<String>,
}

impl SourceFile {
    pub fn new(file_id: FileId, path: PathBuf, content: Arc<String>) -> Self {
        Self {
            file_id,
            path,
            content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_creation() {
        let file_id = FileId::new(1);
        let path = PathBuf::from("/test/file.cpp");
        let content = Arc::new("int main() {}".to_string());

        let source = SourceFile::new(file_id, path.clone(), content.clone());

        assert_eq!(source.file_id, file_id);
        assert_eq!(source.path, path);
        assert_eq!(*source.content, "int main() {}");
    }
}

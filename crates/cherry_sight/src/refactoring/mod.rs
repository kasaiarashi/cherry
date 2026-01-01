// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Refactoring operations
//!
//! Provides automated code refactoring capabilities

pub mod rename;
pub mod extract;
pub mod inline;

pub use rename::{RenameProvider, RenameResult};
pub use extract::{ExtractProvider, ExtractionKind};
pub use inline::InlineProvider;

use crate::util::{FileId, Span};
use std::collections::HashMap;

/// Text edit for refactoring
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub file_id: FileId,
    pub span: Span,
    pub new_text: String,
}

/// Workspace edit containing multiple file edits
#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub changes: HashMap<FileId, Vec<TextEdit>>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }

    pub fn add_edit(&mut self, edit: TextEdit) {
        self.changes
            .entry(edit.file_id)
            .or_default()
            .push(edit);
    }
}

impl Default for WorkspaceEdit {
    fn default() -> Self {
        Self::new()
    }
}

// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP protocol handlers

use crate::db::Database;
use anyhow::Result;
use lsp_types::*;
use std::sync::Arc;

pub mod initialize;
pub mod text_sync;
pub mod completion;
pub mod definition;
pub mod references;
pub mod hover;
pub mod diagnostics;
pub mod rename;

/// Main LSP handlers struct
#[derive(Debug)]
pub struct LspHandlers {
    pub database: Arc<Database>,
}

impl LspHandlers {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Handle textDocument/completion request
    pub fn handle_completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        // Get file content
        let file_id = match self.database.get_file_id(&uri) {
            Some(id) => id,
            None => return Ok(Some(CompletionResponse::Array(vec![]))),
        };

        let _source = match self.database.get_source_file(file_id) {
            Some(s) => s,
            None => return Ok(Some(CompletionResponse::Array(vec![]))),
        };

        log::debug!("Completion request at {}:{}:{}", uri, position.line, position.character);

        // Return basic C++ keywords for now - full provider integration in Phase 18
        let items = vec![
            CompletionItem {
                label: "class".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
            CompletionItem {
                label: "struct".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
            CompletionItem {
                label: "void".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
        ];

        Ok(Some(CompletionResponse::Array(items)))
    }

    /// Handle textDocument/hover request
    pub fn handle_hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;

        log::debug!("Hover request at {}:{}:{}", uri, position.line, position.character);

        // Return basic hover - full provider integration in Phase 18
        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**C++ Symbol**\n\nHover information will be available after Phase 18".to_string(),
            }),
            range: None,
        };

        Ok(Some(hover))
    }

    /// Handle textDocument/definition request
    pub fn handle_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;

        log::debug!("Goto definition request at {}:{}:{}", uri, position.line, position.character);

        // Full provider integration in Phase 18
        Ok(None)
    }

    /// Handle textDocument/references request
    pub fn handle_references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        log::debug!("References request at {}:{}:{}", uri, position.line, position.character);

        // Full provider integration in Phase 18
        Ok(Some(vec![]))
    }

    /// Handle textDocument/didOpen notification
    pub fn handle_did_open(&self, params: DidOpenTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();
        let content = Arc::new(params.text_document.text);

        // Get or create file ID
        let file_id = self.database.get_or_create_file_id(&uri);

        // Parse URI to path
        let path = params.text_document.uri.to_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from(&uri));

        // Add to database
        self.database.add_source_file(file_id, path, content);

        log::info!("Opened file: {} (FileId: {:?})", uri, file_id);
        Ok(())
    }

    /// Handle textDocument/didChange notification
    pub fn handle_did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();

        if let Some(file_id) = self.database.get_file_id(&uri) {
            // For full sync, just use the last change
            if let Some(change) = params.content_changes.last() {
                let content = Arc::new(change.text.clone());
                self.database.update_source_content(file_id, content);
                log::debug!("Updated file: {}", uri);
            }
        }

        Ok(())
    }

    /// Handle textDocument/didSave notification
    pub fn handle_did_save(&self, params: DidSaveTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();
        log::debug!("Saved file: {}", uri);
        Ok(())
    }

    /// Handle textDocument/didClose notification
    pub fn handle_did_close(&self, params: DidCloseTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();

        if let Some(file_id) = self.database.get_file_id(&uri) {
            self.database.remove_source_file(file_id);
            log::info!("Closed file: {}", uri);
        }

        Ok(())
    }
}

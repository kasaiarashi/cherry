// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP protocol handlers

use crate::completion::{CompletionContext, CompletionProvider};
use crate::completion::HoverProvider;
use crate::db::Database;
use crate::index::{AstSymbolBuilder, SymbolId, SymbolTable};
use crate::intelligence::ReferenceFinder;
use crate::lsp::position::{position_to_offset, span_to_range};
use crate::parser::CppParser;
use crate::semantic::{NameResolver, TypeInference};
use crate::util::{FileId, Interner, Position, Span};
use anyhow::Result;
use lsp_types::{self as lsp, *};
use parking_lot::RwLock;
use std::collections::HashMap;
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
pub struct LspHandlers {
    pub database: Arc<Database>,
    symbol_table: Arc<RwLock<SymbolTable>>,
    interner: Arc<RwLock<Interner>>,

    /// Name resolution results per file
    name_resolutions: Arc<RwLock<HashMap<FileId, Arc<crate::semantic::name_resolution::NameResolution>>>>,

    /// Type information per file
    type_info: Arc<RwLock<HashMap<FileId, Arc<HashMap<Span, crate::semantic::type_inference::TypeInfo>>>>>,
}

impl std::fmt::Debug for LspHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspHandlers")
            .field("database", &self.database)
            .finish()
    }
}

impl LspHandlers {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            symbol_table: Arc::new(RwLock::new(SymbolTable::new())),
            interner: Arc::new(RwLock::new(Interner::new())),
            name_resolutions: Arc::new(RwLock::new(HashMap::new())),
            type_info: Arc::new(RwLock::new(HashMap::new())),
        }
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

        // Convert LSP position to our internal Position
        let pos = Position::new(position.line, position.character);

        // Determine completion context
        let context = CompletionContext {
            trigger_character: params.context.as_ref().and_then(|c| c.trigger_character.as_ref().and_then(|s| s.chars().next())),
            is_member_access: params.context.as_ref().and_then(|c| c.trigger_character.as_ref()).map(|s| s == "." || s == "->").unwrap_or(false),
            is_scope_resolution: params.context.as_ref().and_then(|c| c.trigger_character.as_ref()).map(|s| s == "::").unwrap_or(false),
            prefix: String::new(),
        };

        // Get completions from provider
        let symbol_table = self.symbol_table.read();
        let interner = self.interner.read();
        let provider = CompletionProvider::new(&symbol_table, &interner);
        let completions = provider.get_completions(file_id, pos, &context);

        // Convert to LSP completion items
        let items: Vec<CompletionItem> = completions
            .into_iter()
            .map(|item| CompletionItem {
                label: item.label,
                kind: Some(match item.kind {
                    crate::completion::CompletionKind::Function | crate::completion::CompletionKind::Method => CompletionItemKind::FUNCTION,
                    crate::completion::CompletionKind::Class | crate::completion::CompletionKind::Struct => CompletionItemKind::CLASS,
                    crate::completion::CompletionKind::Variable | crate::completion::CompletionKind::Field => CompletionItemKind::VARIABLE,
                    crate::completion::CompletionKind::Keyword => CompletionItemKind::KEYWORD,
                    _ => CompletionItemKind::TEXT,
                }),
                detail: item.detail,
                documentation: item.documentation.map(|d| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: d,
                    })
                }),
                insert_text: item.insert_text,
                sort_text: item.sort_text,
                filter_text: item.filter_text,
                ..Default::default()
            })
            .collect();

        log::debug!("Completion: {} items at {}:{}:{}", items.len(), uri, position.line, position.character);

        Ok(Some(CompletionResponse::Array(items)))
    }

    /// Handle textDocument/hover request
    pub fn handle_hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;

        log::info!("Hover request at {}:{}:{}", uri, position.line, position.character);

        // Get file content
        let file_id = match self.database.get_file_id(&uri) {
            Some(id) => {
                log::info!("Found file_id: {:?} for URI: {}", id, uri);
                id
            },
            None => {
                log::warn!("No file_id found for URI: {}", uri);
                return Ok(None);
            },
        };

        let source = match self.database.get_source_file(file_id) {
            Some(s) => {
                log::info!("Found source file, length: {}", s.content.len());
                s
            },
            None => {
                log::warn!("No source file found for file_id: {:?}", file_id);
                return Ok(None);
            },
        };

        // Convert position to offset
        let offset = match position_to_offset(&source.content, position) {
            Some(o) => {
                log::info!("Position {}:{} converted to offset: {}", position.line, position.character, o);
                o as u32
            },
            None => {
                log::warn!("Failed to convert position {}:{} to offset", position.line, position.character);
                return Ok(None);
            },
        };

        // Find symbol at offset
        let symbol_id = match self.find_symbol_at_offset(file_id, offset) {
            Some(id) => {
                log::info!("Found symbol_id: {} at offset {}", id.0, offset);
                id
            },
            None => {
                log::warn!("No symbol found at offset {} in file {:?}", offset, file_id);
                log::info!("Symbol table has {} symbols total", self.symbol_table.read().len());

                // Debug: log all symbols in this file with their spans
                let symbol_table = self.symbol_table.read();
                let file_symbols = symbol_table.symbols_in_file(file_id);
                log::info!("File has {} symbols:", file_symbols.len());

                for &sym_id in &file_symbols {
                    if let Some(symbol) = symbol_table.get_symbol(sym_id) {
                        let name = self.interner.read().resolve(symbol.name);
                        log::info!("  Symbol #{}: {} (kind: {:?}) span: {}..{}",
                            sym_id.0, name, symbol.kind, symbol.span.start, symbol.span.end);
                    }
                }

                // Debug: check name resolutions
                if let Some(name_res) = self.name_resolutions.read().get(&file_id) {
                    log::info!("Name resolution has {} references", name_res.references.len());
                    for (span, &ref_sym_id) in &name_res.references {
                        if span.start <= offset && offset < span.end {
                            log::info!("  Found reference at {}..{} -> symbol {}",
                                span.start, span.end, ref_sym_id.0);
                        }
                    }
                } else {
                    log::warn!("No name resolution data for this file");
                }

                return Ok(None);
            },
        };

        // Get hover contents from provider
        let symbol_table = self.symbol_table.read();
        let interner = self.interner.read();
        let provider = HoverProvider::new(&symbol_table, &interner);

        let hover_contents = match provider.create_hover_for_symbol(symbol_id) {
            Some(contents) => contents,
            None => {
                log::warn!("Failed to create hover contents for symbol_id: {}", symbol_id.0);
                return Ok(None);
            },
        };

        // Convert to LSP format
        let markdown = hover_contents.contents.join("\n\n");
        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        };

        log::info!("Hover successful: {} at {}:{}:{}", symbol_id.0, uri, position.line, position.character);

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

        // Get file content
        let file_id = match self.database.get_file_id(&uri) {
            Some(id) => id,
            None => return Ok(None),
        };

        let source = match self.database.get_source_file(file_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Convert position to offset
        let offset = match position_to_offset(&source.content, position) {
            Some(o) => o as u32,
            None => return Ok(None),
        };

        // Find symbol at offset
        let symbol_id = match self.find_symbol_at_offset(file_id, offset) {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get definition location
        let symbol_table = self.symbol_table.read();
        let symbol = match symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Get the source file for the definition to convert span to range
        let def_source = match self.database.get_source_file(symbol.file_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Convert span to LSP range
        let range = span_to_range(&def_source.content, symbol.span);

        // Convert file_id to URI
        let def_uri = self.get_uri_for_file_id(symbol.file_id)?;

        log::debug!("Definition: {} at {}:{}:{}", symbol_id.0, def_uri, range.start.line, range.start.character);

        let location = Location {
            uri: def_uri,
            range,
        };

        Ok(Some(GotoDefinitionResponse::Scalar(location)))
    }

    /// Handle textDocument/references request
    pub fn handle_references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        log::debug!("References request at {}:{}:{}", uri, position.line, position.character);

        // Get file content
        let file_id = match self.database.get_file_id(&uri) {
            Some(id) => id,
            None => return Ok(Some(vec![])),
        };

        let source = match self.database.get_source_file(file_id) {
            Some(s) => s,
            None => return Ok(Some(vec![])),
        };

        // Convert position to offset
        let offset = match position_to_offset(&source.content, position) {
            Some(o) => o as u32,
            None => return Ok(Some(vec![])),
        };

        // Find symbol at offset
        let symbol_id = match self.find_symbol_at_offset(file_id, offset) {
            Some(id) => id,
            None => return Ok(Some(vec![])),
        };

        // Find all references
        let symbol_table = self.symbol_table.read();
        let finder = ReferenceFinder::new(&symbol_table);
        let include_declaration = params.context.include_declaration;
        let references = finder.find_references(symbol_id, include_declaration);

        // Convert to LSP locations
        let mut locations = Vec::new();
        for reference in references {
            // Get source file for this reference
            let ref_source = match self.database.get_source_file(reference.file_id) {
                Some(s) => s,
                None => continue,
            };

            // Convert span to range
            let range = span_to_range(&ref_source.content, reference.span);

            // Get URI for this file
            let ref_uri = match self.get_uri_for_file_id(reference.file_id) {
                Ok(uri) => uri,
                Err(_) => continue,
            };

            let location = Location {
                uri: ref_uri,
                range,
            };

            locations.push(location);
        }

        log::debug!("References: {} locations for symbol {}", locations.len(), symbol_id.0);

        Ok(Some(locations))
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
        self.database.add_source_file(file_id, path.clone(), content.clone());

        // Full semantic analysis pipeline
        log::info!("Parsing and analyzing file: {} (FileId: {:?})", uri, file_id);
        match CppParser::new(self.interner.clone()) {
            Ok(mut parser) => {
                match parser.parse(&content, file_id) {
                    Ok(ast) => {
                        log::info!("Parsed {} with {} declarations", uri, ast.declarations.len());

                        // Step 1: Build symbol table from AST
                        let mut builder = AstSymbolBuilder::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                        );
                        builder.build_from_ast(&ast);

                        let symbol_count = self.symbol_table.read().symbols_in_file(file_id).len();
                        log::info!("Built {} symbols for {}", symbol_count, uri);

                        // Step 2: Name resolution
                        let mut name_resolver = NameResolver::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                            file_id,
                        );
                        let name_resolution = name_resolver.resolve(&ast);
                        let ref_count = name_resolution.references.len();
                        log::info!("Resolved {} identifier references", ref_count);

                        // Step 3: Type inference
                        let mut type_inference = TypeInference::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                            name_resolution.clone(),
                        );
                        let types = type_inference.infer(&ast);
                        log::info!("Inferred types for {} expressions", types.len());

                        // Store results
                        self.name_resolutions.write().insert(file_id, Arc::new(name_resolution));
                        self.type_info.write().insert(file_id, Arc::new(types));
                    }
                    Err(e) => {
                        log::warn!("Failed to parse {}: {}", uri, e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to create parser: {}", e);
            }
        }

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

    /// Find the symbol referenced at the given offset (identifier, not containing scope)
    fn find_symbol_at_offset(&self, file_id: FileId, offset: u32) -> Option<SymbolId> {
        // First, try to find a reference at this exact position using name resolution
        if let Some(name_res) = self.name_resolutions.read().get(&file_id) {
            // Look for a reference whose span contains this offset
            for (span, &symbol_id) in &name_res.references {
                if span.file_id == file_id && span.contains(offset) {
                    return Some(symbol_id);
                }
            }
        }

        // Fallback: find the symbol definition at this location
        let symbol_table = self.symbol_table.read();
        let symbols = symbol_table.symbols_in_file(file_id);

        let mut best_match: Option<(SymbolId, u32)> = None;
        let mut candidates = Vec::new();

        for &symbol_id in &symbols {
            if let Some(symbol) = symbol_table.get_symbol(symbol_id) {
                if symbol.span.contains(offset) {
                    let span_size = symbol.span.len();
                    let name = self.interner.read().resolve(symbol.name);
                    candidates.push((symbol_id, name.to_string(), symbol.kind, span_size, symbol.span.start, symbol.span.end));

                    match best_match {
                        None => best_match = Some((symbol_id, span_size)),
                        Some((_, current_size)) if span_size < current_size => {
                            best_match = Some((symbol_id, span_size));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Log all candidates
        if !candidates.is_empty() {
            log::info!("Found {} symbol candidates at offset {}:", candidates.len(), offset);
            for (id, name, kind, size, start, end) in &candidates {
                log::info!("  #{}: {} ({:?}) span={}..{} size={}", id.0, name, kind, start, end, size);
            }
            if let Some((best_id, best_size)) = best_match {
                log::info!("  Best match: #{} (size={})", best_id.0, best_size);
            }
        }

        best_match.map(|(id, _)| id)
    }

    /// Get URI for a FileId by reverse lookup
    fn get_uri_for_file_id(&self, file_id: FileId) -> Result<lsp::Uri> {
        // Get the source file to access its path
        let source = self.database.get_source_file(file_id)
            .ok_or_else(|| anyhow::anyhow!("File not found: {:?}", file_id))?;

        // Convert path to URI
        lsp::Uri::from_file_path(&source.path)
            .map_err(|_| anyhow::anyhow!("Failed to convert path to URI: {:?}", source.path))
    }
}

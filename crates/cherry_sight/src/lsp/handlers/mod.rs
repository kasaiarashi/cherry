// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP protocol handlers

use crate::cache::CacheManager;
use crate::completion::{CompletionContext, CompletionProvider};
use crate::completion::HoverProvider;
use crate::db::Database;
use crate::index::{AstSymbolBuilder, SymbolId, SymbolKind, SymbolTable};
use crate::intelligence::ReferenceFinder;
use crate::lsp::position::{position_to_offset, span_to_range};
use crate::parser::CppParser;
use crate::semantic::{NameResolver, TypeInference};
use crate::util::{FileId, InternedString, Interner, Position, Span};
use anyhow::Result;
use dashmap::DashMap;
use lsp_types::{self as lsp, *};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod initialize;
pub mod text_sync;
pub mod completion;
pub mod definition;
pub mod references;
pub mod hover;
pub mod diagnostics;
pub mod rename;

/// Notification message to send to client
#[derive(Debug, Clone)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

/// Main LSP handlers struct
pub struct LspHandlers {
    pub database: Arc<Database>,
    symbol_table: Arc<RwLock<SymbolTable>>,
    interner: Arc<RwLock<Interner>>,

    /// Name resolution results per file
    name_resolutions: Arc<RwLock<HashMap<FileId, Arc<crate::semantic::name_resolution::NameResolution>>>>,

    /// Type information per file
    type_info: Arc<RwLock<HashMap<FileId, Arc<HashMap<Span, crate::semantic::type_inference::TypeInfo>>>>>,

    /// Workspace root path
    workspace_root: Arc<RwLock<Option<String>>>,

    /// Project indexing status
    indexing_complete: Arc<RwLock<bool>>,

    /// Notification sender
    notification_tx: mpsc::UnboundedSender<Notification>,

    /// PERFORMANCE: Cache for include path resolutions (include_name -> resolved_path)
    include_cache: Arc<DashMap<String, PathBuf>>,

    /// Cache manager for persistent storage
    cache_manager: Arc<RwLock<Option<CacheManager>>>,
}

impl std::fmt::Debug for LspHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspHandlers")
            .field("database", &self.database)
            .finish()
    }
}

impl LspHandlers {
    pub fn new(database: Arc<Database>, notification_tx: mpsc::UnboundedSender<Notification>) -> Self {
        Self {
            database,
            symbol_table: Arc::new(RwLock::new(SymbolTable::new())),
            interner: Arc::new(RwLock::new(Interner::new())),
            name_resolutions: Arc::new(RwLock::new(HashMap::new())),
            type_info: Arc::new(RwLock::new(HashMap::new())),
            workspace_root: Arc::new(RwLock::new(None)),
            indexing_complete: Arc::new(RwLock::new(false)),
            notification_tx,
            include_cache: Arc::new(DashMap::new()),
            cache_manager: Arc::new(RwLock::new(None)),
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

        log::info!("=== GOTO DEFINITION REQUEST ===");
        log::info!("Goto definition request at {}:{}:{}", uri, position.line, position.character);

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

        // First, check if we're on an include directive
        if let Some(include_path) = self.find_include_at_offset(file_id, offset) {
            // Resolve the include path to an actual file
            if let Some(resolved_path) = self.resolve_include_path(&include_path, &source.path) {
                // Try to get the file_id for this path
                let include_uri = lsp::Uri::from_file_path(&resolved_path)
                    .map_err(|_| anyhow::anyhow!("Invalid include path"))?;

                // Return location pointing to the start of the included file
                let location = Location {
                    uri: include_uri,
                    range: lsp::Range {
                        start: lsp::Position { line: 0, character: 0 },
                        end: lsp::Position { line: 0, character: 0 },
                    },
                };

                log::debug!("Include navigation to: {}", resolved_path.display());
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

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

        // Log symbol details
        let symbol_name = self.interner.read().resolve(symbol.name);
        log::info!("Found symbol #{}: {} (kind: {:?})", symbol_id.0, symbol_name, symbol.kind);
        log::info!("  Symbol file: {:?}", symbol.file_id);
        log::info!("  Symbol parent: {:?}", symbol.parent);
        log::info!("  Implementation span: {:?}", symbol.implementation_span);

        // PREFER IMPLEMENTATION: If function has implementation in .cpp, jump there instead of declaration
        let (target_span, target_file_id) = if let Some(impl_span) = symbol.implementation_span {
            log::info!("✓ JUMPING TO IMPLEMENTATION for symbol {} at {:?}", symbol_id.0, impl_span);
            (impl_span, impl_span.file_id)
        } else {
            log::info!("✗ NO IMPLEMENTATION SPAN - jumping to declaration for symbol {}", symbol_id.0);
            (symbol.span, symbol.file_id)
        };

        // Get the source file for the target to convert span to range
        let def_source = match self.database.get_source_file(target_file_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Convert span to LSP range
        let range = span_to_range(&def_source.content, target_span);

        // Convert file_id to URI
        let def_uri = self.get_uri_for_file_id(target_file_id)?;

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

        // Check file type for special handling
        let is_cpp_impl = if let Some(ext) = path.extension() {
            ext == "cpp" || ext == "cc" || ext == "cxx"
        } else {
            false
        };

        // INCREMENTAL: Check if this is a new header file not in cache
        let is_new_header = if let Some(ext) = path.extension() {
            if ext == "h" || ext == "hpp" || ext == "hxx" {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    !self.include_cache.contains_key(file_name)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Add new header to include cache
        if is_new_header {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                self.include_cache.insert(file_name.to_string(), path.clone());
                log::info!("Added new header to include cache: {}", file_name);

                // Save updated cache to disk
                if let Some(cache_mgr) = self.cache_manager.read().as_ref() {
                    let cache_snapshot: HashMap<String, PathBuf> = self.include_cache
                        .iter()
                        .map(|entry| (entry.key().clone(), entry.value().clone()))
                        .collect();

                    if let Err(e) = cache_mgr.save_include_index(&cache_snapshot) {
                        log::warn!("Failed to save updated include cache: {}", e);
                    }
                }
            }
        }

        // CRITICAL: Remove old symbols if this file was already indexed
        // This ensures LSP works when switching between files
        self.symbol_table.write().remove_file_symbols(file_id);
        self.name_resolutions.write().remove(&file_id);
        self.type_info.write().remove(&file_id);

        // Add to database
        self.database.add_source_file(file_id, path.clone(), content.clone());

        // Full semantic analysis pipeline
        log::warn!("CherrySight: Analyzing {} (FileId: {:?})", uri, file_id);
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

                        // IMPLEMENTATION MATCHING: If this is a .cpp file, match implementations to declarations
                        log::info!("File {} is_cpp_impl={}", uri, is_cpp_impl);
                        if is_cpp_impl {
                            log::info!("=== TRIGGERING IMPLEMENTATION MATCHING FOR {} ===", uri);
                            self.match_implementations_to_declarations(file_id);
                        }
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

    /// Match function implementations in .cpp file to their declarations in .h files
    fn match_implementations_to_declarations(&self, cpp_file_id: FileId) {
        log::info!("Matching implementations to declarations for file {:?}", cpp_file_id);

        // Clone necessary data from .cpp symbols
        let cpp_symbol_data: Vec<(SymbolId, InternedString, Option<InternedString>, SymbolKind, Span, Option<SymbolId>)> = {
            let symbol_table = self.symbol_table.read();
            symbol_table.symbols_in_file(cpp_file_id)
                .iter()
                .filter_map(|&id| {
                    let sym = symbol_table.get_symbol(id)?;
                    if sym.kind.is_callable() {
                        Some((sym.id, sym.name, sym.qualified_name, sym.kind, sym.span, sym.parent))
                    } else {
                        None
                    }
                })
                .collect()
        };

        log::info!("Found {} callable symbols in .cpp file", cpp_symbol_data.len());

        let mut updates = Vec::new();

        for (cpp_id, cpp_name, cpp_qualified_name, cpp_kind, cpp_span, cpp_parent) in cpp_symbol_data {
            // Find matching declaration in .h file
            let symbol_table = self.symbol_table.read();
            let interner = self.interner.read();
            let name = interner.resolve(cpp_name);

            log::debug!("Looking for declaration of: {} (kind: {:?})", name, cpp_kind);

            // Find all symbols with same name
            let candidates = symbol_table.find_all(cpp_name);
            log::debug!("  Found {} candidates with name '{}'", candidates.len(), name);

            for &candidate_id in &candidates {
                if let Some(decl_symbol) = symbol_table.get_symbol(candidate_id) {
                    // Skip if it's the same symbol
                    if decl_symbol.id == cpp_id {
                        continue;
                    }

                    // Check if it's a declaration in a .h file
                    if let Some(decl_file) = self.database.get_source_file(decl_symbol.file_id) {
                        if let Some(ext) = decl_file.path.extension() {
                            let is_header = ext == "h" || ext == "hpp" || ext == "hxx";
                            let same_kind = decl_symbol.kind == cpp_kind;

                            log::debug!("  Candidate #{}: is_header={}, same_kind={}, file={:?}",
                                candidate_id.0, is_header, same_kind, decl_file.path);

                            if is_header && same_kind {
                                // For methods, also check if they belong to the same class
                                let same_parent = if cpp_kind == SymbolKind::Method {
                                    // Compare parent class names
                                    if let (Some(cpp_parent_id), Some(decl_parent_id)) = (cpp_parent, decl_symbol.parent) {
                                        if let (Some(cpp_parent_sym), Some(decl_parent_sym)) =
                                            (symbol_table.get_symbol(cpp_parent_id), symbol_table.get_symbol(decl_parent_id)) {
                                            // Compare parent names
                                            cpp_parent_sym.name == decl_parent_sym.name
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    true // For non-methods, we don't need to check parent
                                };

                                if same_parent {
                                    // Found matching declaration!
                                    log::info!("✓ Matched implementation: {} in {:?} -> declaration in {:?}",
                                        name, cpp_file_id, decl_symbol.file_id);

                                    // Store update to apply later
                                    updates.push((candidate_id, cpp_span));
                                    break;
                                } else {
                                    log::debug!("  Skipping - different parent class");
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("Applying {} implementation matches", updates.len());

        // Apply all updates
        let mut symbol_table = self.symbol_table.write();
        for (decl_id, impl_span) in updates {
            if let Some(symbol) = symbol_table.get_symbol_mut(decl_id) {
                symbol.implementation_span = Some(impl_span);
                log::info!("  Updated symbol #{} with implementation span", decl_id.0);
            }
        }
    }

    /// Handle textDocument/didChange notification
    pub fn handle_did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();

        if let Some(file_id) = self.database.get_file_id(&uri) {
            // For full sync, just use the last change
            if let Some(change) = params.content_changes.last() {
                let content = Arc::new(change.text.clone());
                self.database.update_source_content(file_id, content.clone());
                log::debug!("Updated file content: {}", uri);

                // Re-parse and re-index the file to keep symbols in sync
                // Step 1: Remove old symbols for this file
                self.symbol_table.write().remove_file_symbols(file_id);
                log::debug!("Removed old symbols for: {}", uri);

                // Step 2: Parse the new content
                match CppParser::new(self.interner.clone()) {
                    Ok(mut parser) => {
                        match parser.parse(&content, file_id) {
                            Ok(ast) => {
                                log::debug!("Re-parsed {} with {} declarations", uri, ast.declarations.len());

                                // Step 3: Build symbol table from AST
                                let mut builder = AstSymbolBuilder::new(
                                    self.symbol_table.clone(),
                                    self.interner.clone(),
                                );
                                builder.build_from_ast(&ast);

                                let symbol_count = self.symbol_table.read().symbols_in_file(file_id).len();
                                log::debug!("Re-built {} symbols for {}", symbol_count, uri);

                                // Step 4: Name resolution
                                let mut name_resolver = NameResolver::new(
                                    self.symbol_table.clone(),
                                    self.interner.clone(),
                                    file_id,
                                );
                                let name_resolution = name_resolver.resolve(&ast);
                                let ref_count = name_resolution.references.len();
                                log::debug!("Re-resolved {} identifier references", ref_count);

                                // Step 5: Type inference
                                let mut type_inference = TypeInference::new(
                                    self.symbol_table.clone(),
                                    self.interner.clone(),
                                    name_resolution.clone(),
                                );
                                let types = type_inference.infer(&ast);
                                log::debug!("Re-inferred types for {} expressions", types.len());

                                // Store results
                                self.name_resolutions.write().insert(file_id, Arc::new(name_resolution));
                                self.type_info.write().insert(file_id, Arc::new(types));
                            }
                            Err(e) => {
                                log::warn!("Failed to re-parse {}: {}", uri, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create parser for re-parsing: {}", e);
                    }
                }
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

    /// Handle initialization - store workspace info and setup cache
    pub fn handle_initialize(&self, params: lsp::InitializeParams) {
        // Store workspace root
        let root = params.root_uri
            .map(|uri| uri.to_string())
            .or_else(|| params.root_path.clone());

        if let Some(root) = root {
            log::info!("Workspace root: {}", root);
            *self.workspace_root.write() = Some(root.clone());

            // Parse root path for cache manager
            let root_path: Option<std::path::PathBuf> = if root.starts_with("file://") {
                use std::str::FromStr;
                if let Ok(uri) = lsp::Uri::from_str(&root) {
                    uri.to_file_path().ok()
                } else {
                    None
                }
            } else {
                Some(std::path::PathBuf::from(&root))
            };

            // Initialize cache manager and load cached include index
            if let Some(path) = root_path {
                let cache_mgr = CacheManager::new(&path);
                if let Err(e) = cache_mgr.ensure_cache_dir() {
                    log::warn!("Failed to create cache directory: {}", e);
                } else {
                    log::info!("Cache directory: {}", cache_mgr.cache_dir().display());

                    // Try to load cached include index
                    match cache_mgr.load_include_index() {
                        Ok(index) if !index.is_empty() => {
                            log::info!("Loading {} cached include paths...", index.len());
                            CacheManager::populate_dashmap(&self.include_cache, index);
                            log::info!("Loaded include cache - navigation will be instant!");
                        }
                        Ok(_) => {
                            log::info!("No cached include index found - will build during indexing");
                        }
                        Err(e) => {
                            log::warn!("Failed to load include cache: {}", e);
                        }
                    }
                }
                *self.cache_manager.write() = Some(cache_mgr);
            }
        }
    }

    /// Handle initialized notification - start project-wide indexing in background
    pub fn handle_initialized(&self) -> Result<()> {
        log::info!("Client confirmed initialization - starting background indexing");

        let workspace_root = self.workspace_root.read().clone();
        if let Some(root) = workspace_root {
            // Parse the root as a file path or URI
            let root_path: Option<std::path::PathBuf> = if root.starts_with("file://") {
                // Convert URI to path
                use std::str::FromStr;
                if let Ok(uri) = lsp::Uri::from_str(&root) {
                    uri.to_file_path().ok()
                } else {
                    None
                }
            } else {
                Some(std::path::PathBuf::from(&root))
            };

            if let Some(path) = root_path {
                log::info!("Spawning background indexing task from: {}", path.display());

                // Clone what we need for the background task
                let handlers = Self {
                    database: self.database.clone(),
                    symbol_table: self.symbol_table.clone(),
                    interner: self.interner.clone(),
                    name_resolutions: self.name_resolutions.clone(),
                    type_info: self.type_info.clone(),
                    workspace_root: self.workspace_root.clone(),
                    indexing_complete: self.indexing_complete.clone(),
                    notification_tx: self.notification_tx.clone(),
                    include_cache: self.include_cache.clone(),
                    cache_manager: self.cache_manager.clone(),
                };

                // Spawn indexing in background thread (not async task to avoid blocking tokio runtime)
                std::thread::spawn(move || {
                    log::info!("Background indexing thread started");
                    match handlers.index_project(&path) {
                        Ok(indexed_files) => {
                            *handlers.indexing_complete.write() = true;
                            log::info!("Background indexing complete! Indexed {} files", indexed_files.len());

                            // Build complete include index (project + engine headers)
                            log::info!("Building complete include index...");
                            let mut all_headers = indexed_files.clone();

                            // Discover engine headers too
                            if let Some(engine_paths) = handlers.get_ue5_engine_paths() {
                                for engine_path in &engine_paths {
                                    let mut engine_headers = Vec::new();
                                    if handlers.discover_cpp_files(engine_path, &mut engine_headers).is_ok() {
                                        log::info!("Found {} engine headers in {}", engine_headers.len(), engine_path.display());
                                        all_headers.extend(engine_headers);
                                    }
                                }
                            }

                            log::info!("Total headers for include index: {}", all_headers.len());

                            // Build and save include index
                            let include_index = CacheManager::build_include_index(&all_headers);

                            // Populate runtime cache
                            CacheManager::populate_dashmap(&handlers.include_cache, include_index.clone());

                            // Save to disk for next startup
                            if let Some(cache_mgr) = handlers.cache_manager.read().as_ref() {
                                if let Err(e) = cache_mgr.save_metadata(&indexed_files) {
                                    log::warn!("Failed to save cache metadata: {}", e);
                                }

                                if let Err(e) = cache_mgr.save_include_index(&include_index) {
                                    log::warn!("Failed to save include index: {}", e);
                                } else {
                                    log::info!("Saved include index to .cherry/includes.bin - navigation will be instant on restart!");
                                }
                            }

                            // MATCH IMPLEMENTATIONS: Link .cpp implementations to .h declarations
                            log::info!("Matching implementations to declarations for all .cpp files...");
                            let cpp_files: Vec<FileId> = indexed_files.iter()
                                .filter_map(|path| {
                                    if let Some(ext) = path.extension() {
                                        if ext == "cpp" || ext == "cc" || ext == "cxx" {
                                            // Convert path to URI to get file_id
                                            if let Ok(uri) = lsp::Uri::from_file_path(path) {
                                                handlers.database.get_file_id(&uri.to_string())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            log::info!("Found {} .cpp files to process", cpp_files.len());
                            for cpp_file_id in cpp_files {
                                handlers.match_implementations_to_declarations(cpp_file_id);
                            }
                            log::info!("Implementation matching complete!");
                        }
                        Err(e) => {
                            log::error!("Background indexing failed: {}", e);
                        }
                    }
                });
            }
        } else {
            log::warn!("No workspace root - skipping project indexing");
        }

        Ok(())
    }

    /// Send a progress notification to the client
    fn send_progress(&self, token: &str, kind: &str, title: Option<&str>, message: Option<&str>, percentage: Option<u32>) {
        let value = match kind {
            "begin" => {
                serde_json::json!({
                    "kind": "begin",
                    "title": title.unwrap_or("CherrySight"),
                    "message": message,
                    "percentage": percentage,
                })
            }
            "report" => {
                serde_json::json!({
                    "kind": "report",
                    "message": message,
                    "percentage": percentage,
                })
            }
            "end" => {
                serde_json::json!({
                    "kind": "end",
                    "message": message,
                })
            }
            _ => return,
        };

        let params = serde_json::json!({
            "token": token,
            "value": value,
        });

        let notification = Notification {
            method: "$/progress".to_string(),
            params,
        };

        // Send notification (log if it fails)
        if let Err(e) = self.notification_tx.send(notification) {
            log::warn!("Failed to send progress notification: {}", e);
        } else {
            log::debug!("Sent progress notification: {} - {:?}", kind, message);
        }
    }

    /// Index all C++ files in the project (with parallel processing)
    /// Returns the list of indexed file paths for caching
    fn index_project(&self, root: &std::path::Path) -> Result<Vec<PathBuf>> {
        use rayon::prelude::*;
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Progress token - use a unique identifier
        let token = "cherry-sight-index";

        // Send begin progress with clear title
        self.send_progress(
            token,
            "begin",
            Some("Indexing Project"),
            Some("Discovering header files..."),
            Some(0),
        );

        // Find all .h files (headers only for performance)
        let mut files_to_index = Vec::new();
        self.discover_cpp_files(root, &mut files_to_index)?;

        log::info!("Found {} header files to index", files_to_index.len());

        let total_files = files_to_index.len();
        let indexed_count = Arc::new(AtomicUsize::new(0));

        // PERFORMANCE OPTIMIZATION: Use rayon for parallel file processing
        // Process files in parallel batches to maximize CPU utilization
        let batch_size = 50; // Send progress every 50 files

        for (_batch_idx, chunk) in files_to_index.chunks(batch_size).enumerate() {
            let chunk_results: Vec<_> = chunk.par_iter().filter_map(|file_path| {
                // Read file
                let content = match fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to read {}: {}", file_path.display(), e);
                        return None;
                    }
                };

                // Create URI
                let uri = match lsp::Uri::from_file_path(file_path) {
                    Ok(u) => u.to_string(),
                    Err(_) => {
                        log::warn!("Invalid file path: {}", file_path.display());
                        return None;
                    }
                };

                Some((file_path.clone(), uri, content))
            }).collect();

            // Process results sequentially to avoid lock contention
            for (file_path, uri, content) in chunk_results {
                let file_id = self.database.get_or_create_file_id(&uri);
                let content_arc = Arc::new(content);
                self.database.add_source_file(file_id, file_path.clone(), content_arc.clone());

                // Parse and index
                if let Err(e) = self.parse_and_index_file(file_id, &content_arc) {
                    log::warn!("Failed to index {}: {}", file_path.display(), e);
                }

                let current = indexed_count.fetch_add(1, Ordering::Relaxed) + 1;

                // Send progress report every file
                let percentage = ((current as f64 / total_files as f64) * 100.0) as u32;
                let file_name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                self.send_progress(
                    token,
                    "report",
                    None,
                    Some(&format!("Indexed {}/{}: {}", current, total_files, file_name)),
                    Some(percentage),
                );
            }
        }

        // Send end progress
        self.send_progress(
            token,
            "end",
            None,
            Some(&format!("Indexed {} header files", total_files)),
            None,
        );

        log::info!("Indexed {} header files successfully", total_files);
        Ok(files_to_index)
    }

    /// Recursively discover C++ files (headers only for performance)
    fn discover_cpp_files(&self, dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
        use std::fs;

        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden directories and common build directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "build" || name == "Build" ||
                   name == "Binaries" || name == "Intermediate" || name == "DerivedDataCache" {
                    continue;
                }
            }

            if path.is_dir() {
                self.discover_cpp_files(&path, files)?;
            } else if let Some(ext) = path.extension() {
                // PERFORMANCE OPTIMIZATION: Only index headers initially
                // Headers contain declarations which are what we need for most IDE features
                // .cpp files are much larger and slower to parse
                if ext == "h" || ext == "hpp" || ext == "hxx" {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    /// Parse and index a single file
    fn parse_and_index_file(&self, file_id: FileId, content: &Arc<String>) -> Result<()> {
        match CppParser::new(self.interner.clone()) {
            Ok(mut parser) => {
                match parser.parse(content, file_id) {
                    Ok(ast) => {
                        // Step 1: Build symbol table from AST
                        let mut builder = AstSymbolBuilder::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                        );
                        builder.build_from_ast(&ast);

                        // Step 2: Name resolution
                        let mut name_resolver = NameResolver::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                            file_id,
                        );
                        let name_resolution = name_resolver.resolve(&ast);

                        // Step 3: Type inference
                        let mut type_inference = TypeInference::new(
                            self.symbol_table.clone(),
                            self.interner.clone(),
                            name_resolution.clone(),
                        );
                        let types = type_inference.infer(&ast);

                        // Store results
                        self.name_resolutions.write().insert(file_id, Arc::new(name_resolution));
                        self.type_info.write().insert(file_id, Arc::new(types));

                        Ok(())
                    }
                    Err(e) => {
                        log::warn!("Failed to parse file {:?}: {}", file_id, e);
                        Ok(()) // Don't fail the whole indexing
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to create parser: {}", e);
                Ok(()) // Don't fail the whole indexing
            }
        }
    }

    /// Find include directive at the given offset
    fn find_include_at_offset(&self, file_id: FileId, offset: u32) -> Option<String> {
        // Parse the file to get includes
        let source = self.database.get_source_file(file_id)?;
        match CppParser::new(self.interner.clone()) {
            Ok(mut parser) => match parser.parse(&source.content, file_id) {
                Ok(ast) => {
                    // Find include whose span contains the offset
                    for include in &ast.includes {
                        if include.span.contains(offset) {
                            return Some(include.path.clone());
                        }
                    }
                    None
                }
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    /// Resolve an include path to an actual file path (with instant cached lookup)
    fn resolve_include_path(&self, include_path: &str, current_file: &std::path::Path) -> Option<std::path::PathBuf> {
        // Try relative to current file first
        if let Some(parent) = current_file.parent() {
            let relative_path = parent.join(include_path);
            if relative_path.exists() {
                return Some(relative_path);
            }
        }

        // INSTANT LOOKUP: Check cached index by filename
        // Extract just the filename from the include path
        let file_name = std::path::Path::new(include_path)
            .file_name()
            .and_then(|n| n.to_str())?;

        // Try exact filename match from cache (instant!)
        if let Some(cached_path) = self.include_cache.get(file_name) {
            log::debug!("Include cache HIT: {} -> {}", include_path, cached_path.display());
            return Some(cached_path.clone());
        }

        // If not in cache yet, try workspace-relative search
        log::warn!("Include cache MISS: {} - will search filesystem", include_path);

        if let Some(root) = self.workspace_root.read().as_ref() {
            let root_path: std::path::PathBuf = if root.starts_with("file://") {
                use std::str::FromStr;
                if let Ok(uri) = lsp::Uri::from_str(root) {
                    uri.to_file_path().ok()?
                } else {
                    std::path::PathBuf::from(root.trim_start_matches("file://"))
                }
            } else {
                std::path::PathBuf::from(root)
            };

            // Search in common UE5 project directories
            let mut search_paths = vec![
                root_path.join("Source"),
                root_path.join("Plugins"),
                root_path.clone(),
            ];

            // Add UE5 engine source paths
            if let Some(engine_paths) = self.get_ue5_engine_paths() {
                search_paths.extend(engine_paths);
            }

            for search_path in search_paths {
                if let Ok(found) = self.search_for_include(&search_path, include_path) {
                    // Cache the successful resolution for next time
                    self.include_cache.insert(file_name.to_string(), found.clone());
                    log::info!("Cached new include: {} -> {}", file_name, found.display());
                    return Some(found);
                }
            }
        }

        None
    }

    /// Get UE5 engine source paths from common install locations
    fn get_ue5_engine_paths(&self) -> Option<Vec<std::path::PathBuf>> {
        let mut paths = Vec::new();

        // Try UE_ROOT environment variable first
        if let Ok(ue_root) = std::env::var("UE_ROOT") {
            let engine_path = std::path::PathBuf::from(ue_root);
            paths.push(engine_path.join("Engine/Source/Runtime"));
            paths.push(engine_path.join("Engine/Source/Editor"));
            paths.push(engine_path.join("Engine/Source/Developer"));
            paths.push(engine_path.join("Engine/Plugins"));
            paths.push(engine_path.join("Engine/Source"));
        }

        // Try common Windows install locations
        let common_locations = vec![
            "C:/Program Files/Epic Games",
            "D:/Program Files/Epic Games",
            "W:/Softwares",
        ];

        for base in common_locations {
            let base_path = std::path::Path::new(base);
            if base_path.exists() {
                // Look for UE_5.x directories
                if let Ok(entries) = std::fs::read_dir(base_path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("UE_5") || name_str.starts_with("UnrealEngine-5") {
                            let engine_path = entry.path();
                            paths.push(engine_path.join("Engine/Source/Runtime"));
                            paths.push(engine_path.join("Engine/Source/Editor"));
                            paths.push(engine_path.join("Engine/Source/Developer"));
                            paths.push(engine_path.join("Engine/Plugins"));
                            paths.push(engine_path.join("Engine/Source"));
                        }
                    }
                }
            }
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }

    /// Recursively search for an include file
    fn search_for_include(&self, base_path: &std::path::Path, include_name: &str) -> Result<std::path::PathBuf> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(base_path).max_depth(10).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(file_name) = entry.path().file_name() {
                    if file_name == include_name {
                        return Ok(entry.path().to_path_buf());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Include file not found"))
    }
}

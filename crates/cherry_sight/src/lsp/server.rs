// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP server core

use crate::db::Database;
use crate::lsp::handlers::LspHandlers;
use anyhow::{Context, Result};
use lsp_types::*;
use serde_json::{from_value, to_string, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// LSP server for cherry-sight
#[derive(Debug)]
pub struct LspServer {
    handlers: Arc<LspHandlers>,
}

impl LspServer {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            handlers: Arc::new(LspHandlers::new(database)),
        }
    }

    /// Get the LSP handlers for this server
    pub fn handlers(&self) -> Arc<LspHandlers> {
        self.handlers.clone()
    }

    /// Start the LSP server (for standalone mode)
    pub async fn run(&self) {
        log::info!("Cherry-Sight LSP server started");

        if let Err(e) = self.run_loop().await {
            log::error!("LSP server error: {}", e);
        }

        log::info!("Cherry-Sight LSP server stopped");
    }

    async fn run_loop(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdin = tokio::io::BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        loop {
            // Read Content-Length header
            let mut header = String::new();
            if stdin.read_line(&mut header).await? == 0 {
                log::info!("EOF on stdin, shutting down");
                break;
            }

            let content_length = header
                .trim()
                .strip_prefix("Content-Length: ")
                .context("Missing Content-Length header")?
                .parse::<usize>()
                .context("Invalid Content-Length")?;

            // Read empty line
            let mut empty = String::new();
            stdin.read_line(&mut empty).await?;

            // Read message body
            let mut body = vec![0u8; content_length];
            tokio::io::AsyncReadExt::read_exact(&mut stdin, &mut body).await?;

            let message = String::from_utf8(body)?;
            log::debug!("Received: {}", message);

            // Parse JSON-RPC message
            let json: Value = serde_json::from_str(&message)?;

            // Handle the message
            if let Some(response) = self.handle_message(json).await? {
                let response_str = to_string(&response)?;
                log::debug!("Sending: {}", response_str);

                // Write response
                let response_bytes = response_str.as_bytes();
                stdout
                    .write_all(format!("Content-Length: {}\r\n\r\n", response_bytes.len()).as_bytes())
                    .await?;
                stdout.write_all(response_bytes).await?;
                stdout.flush().await?;
            }
        }

        Ok(())
    }

    async fn handle_message(&self, message: Value) -> Result<Option<Value>> {
        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        log::debug!("Method: {}, ID: {:?}", method, id);

        match method {
            "initialize" => {
                let _init_params: InitializeParams = from_value(params)?;
                let result = InitializeResult {
                    capabilities: ServerCapabilities {
                        text_document_sync: Some(TextDocumentSyncCapability::Options(
                            TextDocumentSyncOptions {
                                open_close: Some(true),
                                change: Some(TextDocumentSyncKind::INCREMENTAL),
                                ..Default::default()
                            },
                        )),
                        completion_provider: Some(CompletionOptions {
                            trigger_characters: Some(vec![
                                ".".to_string(),
                                "->".to_string(),
                                "::".to_string(),
                            ]),
                            ..Default::default()
                        }),
                        hover_provider: Some(HoverProviderCapability::Simple(true)),
                        definition_provider: Some(OneOf::Left(true)),
                        references_provider: Some(OneOf::Left(true)),
                        ..Default::default()
                    },
                    server_info: Some(ServerInfo {
                        name: "cherry-sight".to_string(),
                        version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    }),
                    ..Default::default()
                };
                Ok(Some(self.make_response(id, serde_json::to_value(result)?)))
            }
            "initialized" => {
                log::info!("Client confirmed initialization");
                Ok(None)
            }
            "shutdown" => {
                log::info!("Shutdown request received");
                Ok(Some(self.make_response(id, Value::Null)))
            }
            "exit" => {
                log::info!("Exit notification received");
                std::process::exit(0);
            }
            "textDocument/didOpen" => {
                let params: DidOpenTextDocumentParams = from_value(params)?;
                self.handlers.handle_did_open(params)?;
                Ok(None)
            }
            "textDocument/didChange" => {
                let params: DidChangeTextDocumentParams = from_value(params)?;
                self.handlers.handle_did_change(params)?;
                Ok(None)
            }
            "textDocument/didSave" => {
                let params: DidSaveTextDocumentParams = from_value(params)?;
                self.handlers.handle_did_save(params)?;
                Ok(None)
            }
            "textDocument/didClose" => {
                let params: DidCloseTextDocumentParams = from_value(params)?;
                self.handlers.handle_did_close(params)?;
                Ok(None)
            }
            "textDocument/completion" => {
                let params: CompletionParams = from_value(params)?;
                let result = self.handlers.handle_completion(params)?;
                Ok(Some(self.make_response(id, serde_json::to_value(result)?)))
            }
            "textDocument/hover" => {
                let params: HoverParams = from_value(params)?;
                let result = self.handlers.handle_hover(params)?;
                Ok(Some(self.make_response(id, serde_json::to_value(result)?)))
            }
            "textDocument/definition" => {
                let params: GotoDefinitionParams = from_value(params)?;
                let result = self.handlers.handle_goto_definition(params)?;
                Ok(Some(self.make_response(id, serde_json::to_value(result)?)))
            }
            "textDocument/references" => {
                let params: ReferenceParams = from_value(params)?;
                let result = self.handlers.handle_references(params)?;
                Ok(Some(self.make_response(id, serde_json::to_value(result)?)))
            }
            _ => {
                log::warn!("Unhandled method: {}", method);
                Ok(None)
            }
        }
    }

    fn make_response(&self, id: Option<Value>, result: Value) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new(Arc::new(Database::new()))
    }
}

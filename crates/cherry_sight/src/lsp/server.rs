// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP server core

use crate::db::Database;
use crate::lsp::handlers::LspHandlers;
use std::sync::Arc;

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
        // In standalone mode, would read from stdin/write to stdout
        // For built-in mode (Cherry IDE), this is not used
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new(Arc::new(Database::new()))
    }
}

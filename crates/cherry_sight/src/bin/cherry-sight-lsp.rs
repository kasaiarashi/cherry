// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Cherry-Sight LSP Server Binary
//!
//! This binary provides a standalone Language Server Protocol server for C++ and UE5.

use cherry_sight::{Database, lsp::LspServer};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    log::info!("Cherry-Sight LSP Server starting...");

    // Create database
    let database = Arc::new(Database::new());

    // Create LSP server
    let server = LspServer::new(database);

    // Run server (reads from stdin, writes to stdout)
    server.run().await;

    log::info!("Cherry-Sight LSP Server shutdown");
}

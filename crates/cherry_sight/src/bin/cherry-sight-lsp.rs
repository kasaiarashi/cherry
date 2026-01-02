// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Cherry-Sight LSP Server Binary
//!
//! This binary provides a standalone Language Server Protocol server for C++ and UE5.

use cherry_sight::{Database, lsp::LspServer};
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::SystemTime;

#[tokio::main]
async fn main() {
    // Initialize logging to both stderr and file
    // Write log file next to the binary executable for consistent location
    let log_file_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .join("cherry-sight-lsp.log");

    let log_file_path_clone = log_file_path.clone();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(move |buf, record| {
            // Write to stderr (for editor)
            writeln!(buf, "[{}] {}", record.level(), record.args())?;

            // Also write to file
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path_clone)
            {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let _ = writeln!(
                    file,
                    "[{} {} {}:{}] {}",
                    now,
                    record.level(),
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.args()
                );
            }

            Ok(())
        })
        .init();

    log::info!("Cherry-Sight LSP Server starting...");
    log::info!("Logging to file: {}", log_file_path.display());

    // Create database
    let database = Arc::new(Database::new());

    // Create LSP server
    let server = LspServer::new(database);

    // Run server (reads from stdin, writes to stdout)
    server.run().await;

    log::info!("Cherry-Sight LSP Server shutdown");
}

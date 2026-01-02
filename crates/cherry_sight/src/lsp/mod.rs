// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP server implementation
//!
//! This module will contain:
//! - LSP server core
//! - Protocol handlers
//! - Capabilities management
//!
//! To be implemented in Phase 4-5

pub mod server;
pub mod capabilities;
pub mod handlers;

pub use server::LspServer;
pub use capabilities::ServerCapabilities;
pub use handlers::LspHandlers;

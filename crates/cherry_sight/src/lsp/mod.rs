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
pub mod position;

pub use server::LspServer;
pub use capabilities::ServerCapabilities;
pub use handlers::LspHandlers;
pub use position::{offset_to_position, position_to_offset, range_to_span, span_to_range};

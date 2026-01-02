// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Cherry-Sight: C++ code intelligence for Unreal Engine 5
//!
//! A pure Rust implementation of a complete code intelligence system for C++ and UE5,
//! featuring incremental parsing, semantic analysis, and LSP server capabilities.

// Module declarations
pub mod ast;
pub mod parser;
pub mod index;
pub mod db;
pub mod ue;
pub mod lsp;
pub mod types;
pub mod util;
pub mod intelligence;
pub mod completion;
pub mod diagnostics;
pub mod refactoring;
pub mod codegen;
pub mod ue_integration;

// Re-export commonly used types
pub use ast::{TranslationUnit, Declaration};
pub use parser::CppParser;
pub use db::Database;
pub use ue::UEProject;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

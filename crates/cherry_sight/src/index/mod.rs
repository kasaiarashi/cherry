// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol indexing and name resolution
//!
//! This module will contain:
//! - Symbol definitions and symbol tables
//! - Hierarchical scope management
//! - Name resolution (qualified and unqualified)
//! - Cross-reference tracking
//!
//! To be implemented in Phase 2

pub mod symbol;
pub mod symbol_table;
pub mod scope;
pub mod name_resolution;

pub use symbol::{SymbolId, SymbolKind, Symbol};
pub use symbol_table::SymbolTable;
pub use scope::Scope;

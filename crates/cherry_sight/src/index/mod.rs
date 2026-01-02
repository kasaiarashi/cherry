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
pub mod ast_to_symbols;

pub use symbol::{SymbolId, SymbolKind, Symbol, Visibility, SymbolFlags};
pub use symbol_table::SymbolTable;
pub use scope::{Scope, ScopeKind, ScopeStack};
pub use name_resolution::NameResolver;
pub use ast_to_symbols::AstSymbolBuilder;

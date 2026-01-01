// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol definitions
//!
//! To be implemented in Phase 2

use crate::util::{InternedString, Span};

/// Unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// The kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Namespace,
    Class,
    Struct,
    Enum,
    Function,
    Variable,
    Parameter,
    Field,
    EnumVariant,
    TypeAlias,
    Template,
}

/// A symbol in the code
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub name: InternedString,
    pub span: Span,
    pub parent: Option<SymbolId>,
}

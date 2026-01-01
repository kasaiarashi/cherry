// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Type definitions and representations
//!
//! To be implemented in Phase 2

use crate::util::InternedString;

/// Type definition placeholder
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    pub name: InternedString,
}

impl TypeDef {
    pub fn new(name: InternedString) -> Self {
        Self { name }
    }
}

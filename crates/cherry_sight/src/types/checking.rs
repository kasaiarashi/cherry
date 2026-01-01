// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Type checking and compatibility
//!
//! To be implemented in Phase 2

use crate::types::TypeDef;

/// Type checker placeholder
#[derive(Debug, Default)]
pub struct TypeChecker {}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_compatible(&self, _t1: &TypeDef, _t2: &TypeDef) -> bool {
        false
    }
}

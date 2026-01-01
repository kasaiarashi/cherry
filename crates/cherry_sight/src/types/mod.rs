// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Type system for C++
//!
//! This module will contain:
//! - Type definitions and representations
//! - Type inference
//! - Type checking and compatibility
//! - Template type deduction
//!
//! To be implemented in Phase 2

pub mod type_def;
pub mod inference;
pub mod checking;

pub use type_def::TypeDef;
pub use inference::TypeInference;
pub use checking::TypeChecker;

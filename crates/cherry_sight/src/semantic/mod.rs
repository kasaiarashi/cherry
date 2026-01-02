// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Semantic analysis - name resolution and type inference

pub mod name_resolution;
pub mod type_inference;
pub mod scope;

pub use name_resolution::NameResolver;
pub use type_inference::{TypeInference, TypeInfo};
pub use scope::{Scope, ScopeId, ScopeKind};

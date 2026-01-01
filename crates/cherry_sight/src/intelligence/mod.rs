// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Code intelligence features
//!
//! This module provides:
//! - Go to definition
//! - Go to declaration
//! - Find all references
//! - Find implementations
//! - Symbol search
//! - Type hierarchy
//! - Call hierarchy

pub mod definition;
pub mod references;
pub mod implementations;
pub mod search;
pub mod hierarchy;

pub use definition::{DefinitionFinder, DefinitionResult};
pub use references::{ReferenceFinder, ReferenceResult};
pub use implementations::{ImplementationFinder, ImplementationResult};
pub use search::{SymbolSearcher, SearchResult};
pub use hierarchy::{TypeHierarchyBuilder, CallHierarchyBuilder};

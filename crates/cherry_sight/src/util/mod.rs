// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Utility modules for cherry-sight

pub mod interning;
pub mod span;

pub use interning::{Interner, InternedString};
pub use span::{FileId, Span, Position, TextRange};

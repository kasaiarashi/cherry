// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! C++ parser using tree-sitter-cpp

pub mod cpp_parser;
pub mod error_recovery;
pub mod incremental;
pub mod text_range;

pub use cpp_parser::CppParser;
pub use error_recovery::collect_errors;
pub use incremental::IncrementalParser;
pub use text_range::TextRangeExt;

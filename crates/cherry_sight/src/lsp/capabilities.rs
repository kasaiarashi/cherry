// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP server capabilities
//!
//! To be implemented in Phase 4-5

/// Server capabilities placeholder
#[derive(Debug, Default)]
pub struct ServerCapabilities {
    pub text_document_sync: bool,
    pub completion: bool,
    pub hover: bool,
    pub definition: bool,
    pub references: bool,
}

impl ServerCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn full() -> Self {
        Self {
            text_document_sync: true,
            completion: true,
            hover: true,
            definition: true,
            references: true,
        }
    }
}

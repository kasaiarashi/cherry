// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Code assistance and autocompletion features
//!
//! This module provides:
//! - Context-aware autocompletion
//! - Member access completions (., ->, ::)
//! - Function parameter hints
//! - Signature help
//! - Hover information
//! - UE5-specific completions

pub mod provider;
pub mod signature;
pub mod hover;
pub mod ue_completions;

pub use provider::{CompletionProvider, CompletionItem, CompletionKind, CompletionContext};
pub use signature::{SignatureHelp, SignatureInformation, ParameterInformation};
pub use hover::{HoverProvider, HoverContents};
pub use ue_completions::UE5CompletionProvider;

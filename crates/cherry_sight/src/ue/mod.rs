// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Unreal Engine 5 project structure and integration
//!
//! This module handles:
//! - .uproject file parsing
//! - .uplugin file parsing
//! - Module structure and dependencies
//! - Build.cs file parsing
//! - UE5-specific macros and reflection

pub mod project;
pub mod module;
pub mod plugin;
pub mod build_cs;
pub mod macros;
pub mod generated_headers;

pub use project::UEProject;
pub use module::{UEModule, ModuleType};
pub use plugin::UEPlugin;
pub use macros::UEMacroKind;

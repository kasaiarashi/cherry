// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 deep integration features

mod module_graph;
mod blueprint_refs;
mod asset_tracker;

pub use module_graph::{ModuleDependencyGraph, ModuleDependency, DependencyKind};
pub use blueprint_refs::{BlueprintRefTracker, BlueprintReference, BlueprintRefKind};
pub use asset_tracker::{AssetTracker, AssetReference, AssetKind};

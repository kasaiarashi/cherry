// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Project-wide analysis and metrics

mod dead_code;
mod complexity;
mod dependencies;

pub use dead_code::{CodeLocation, DeadCodeDetector, DeadCodeResult, DeadCodeStats};
pub use complexity::{
    ComplexityAnalyzer, ComplexityMetrics, ComplexityRating, FileComplexityMetrics,
    MaintainabilityIndex, MaintainabilityRating, ProjectComplexityReport,
};
pub use dependencies::{
    CouplingMetrics, DependencyAnalyzer, DependencyGraph, DependencyViolation, ViolationKind,
};

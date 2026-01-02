// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Error recovery and robustness framework

mod graceful;
mod partial;
mod thread_safe;

pub use graceful::{
    ErrorRecoveryManager, ErrorSeverity, RecoverableError, RecoveryStrategy,
};
pub use partial::{
    AnalysisStatus, CompletenessStats, CompletenessTracker, PartialAnalysisResult,
    PartialAnalyzer,
};
pub use thread_safe::{
    AnalysisLock, AnalysisProgress, AnalysisStage, ResultCache, ThreadSafeAnalyzer,
};

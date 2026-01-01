mod connection;
mod project_detection;
mod protocol;
mod settings;
pub mod solution;
pub mod ue_project;
pub mod vcxproj;

pub use connection::CherryLinkConnection;
pub use project_detection::{find_uproject_name, find_uproject_path, is_unreal_project};
pub use protocol::{BuildConfiguration, BuildStatus, LogMessage, PlayState};
pub use settings::CherryLinkSettings;
pub use solution::{
    ParsedSolution, SlnBuildConfiguration, SlnNestedProject, SlnProject, VsProjectType,
    find_solution_file, parse_solution,
};
pub use vcxproj::{
    DirectoryNode, ParsedVcxproj, VcxprojFile, VcxprojFileType, parse_csproj, parse_vcxproj,
};

use gpui::{App, EventEmitter, Global};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Global state for UE project information
/// This allows different parts of the application to share project info
#[derive(Default)]
pub struct UnrealProjectInfo {
    /// Path to the UE Engine folder
    pub engine_path: Option<PathBuf>,
    /// Path to the .uproject file
    pub project_path: Option<PathBuf>,
    /// Build configurations from solution (e.g., "Development Editor", "DebugGame")
    pub configurations: Vec<String>,
    /// Platforms from solution (e.g., "Win64", "Linux")
    pub platforms: Vec<String>,
}

/// Thread-safe wrapper for UnrealProjectInfo
#[derive(Clone, Default)]
pub struct UnrealProjectInfoGlobal(pub Arc<RwLock<UnrealProjectInfo>>);

impl Global for UnrealProjectInfoGlobal {}

impl UnrealProjectInfoGlobal {
    pub fn set_engine_path(&self, path: Option<PathBuf>) {
        if let Ok(mut info) = self.0.write() {
            info.engine_path = path;
        }
    }

    pub fn set_project_path(&self, path: Option<PathBuf>) {
        if let Ok(mut info) = self.0.write() {
            info.project_path = path;
        }
    }

    pub fn engine_path(&self) -> Option<PathBuf> {
        self.0.read().ok().and_then(|info| info.engine_path.clone())
    }

    pub fn project_path(&self) -> Option<PathBuf> {
        self.0.read().ok().and_then(|info| info.project_path.clone())
    }

    pub fn set_configurations(&self, configs: Vec<String>) {
        if let Ok(mut info) = self.0.write() {
            info.configurations = configs;
        }
    }

    pub fn set_platforms(&self, platforms: Vec<String>) {
        if let Ok(mut info) = self.0.write() {
            info.platforms = platforms;
        }
    }

    pub fn configurations(&self) -> Vec<String> {
        self.0.read().ok().map(|info| info.configurations.clone()).unwrap_or_default()
    }

    pub fn platforms(&self) -> Vec<String> {
        self.0.read().ok().map(|info| info.platforms.clone()).unwrap_or_default()
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(UnrealProjectInfoGlobal::default());
}

/// Events emitted by CherryLink connection
#[derive(Clone, Debug)]
pub enum CherryLinkEvent {
    Connected,
    Disconnected,
    PlayStateChanged(PlayState),
    BuildStatusChanged(BuildStatus),
    LogReceived(LogMessage),
}

impl EventEmitter<CherryLinkEvent> for CherryLinkConnection {}

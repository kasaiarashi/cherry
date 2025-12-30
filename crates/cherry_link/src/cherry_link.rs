mod connection;
mod project_detection;
mod protocol;
mod settings;

pub use connection::CherryLinkConnection;
pub use project_detection::{find_uproject_name, find_uproject_path, is_unreal_project};
pub use protocol::{BuildConfiguration, BuildStatus, LogMessage, PlayState};
pub use settings::CherryLinkSettings;

use gpui::{App, EventEmitter};

pub fn init(_cx: &mut App) {
    // CherryLink settings will be registered when integrated with the settings store
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

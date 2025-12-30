use crate::protocol::{BuildConfiguration, BuildStatus, LogMessage, PlayState};
use crate::CherryLinkEvent;
use gpui::{App, AsyncApp, Context, Entity, Task, WeakEntity};

/// Connection state for CherryLink
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

/// CherryLink connection entity
pub struct CherryLinkConnection {
    state: ConnectionState,
    port: u16,
    play_state: PlayState,
    build_status: BuildStatus,
    selected_config: BuildConfiguration,
}

impl CherryLinkConnection {
    pub fn new(port: u16) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            port,
            play_state: PlayState::default(),
            build_status: BuildStatus::default(),
            selected_config: BuildConfiguration::default(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state
    }

    pub fn play_state(&self) -> PlayState {
        self.play_state
    }

    pub fn build_status(&self) -> BuildStatus {
        self.build_status
    }

    pub fn selected_configuration(&self) -> BuildConfiguration {
        self.selected_config
    }

    pub fn set_selected_configuration(&mut self, config: BuildConfiguration, cx: &mut Context<Self>) {
        self.selected_config = config;
        cx.notify();
    }

    /// Connect to the CherryLink server
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        if self.state != ConnectionState::Disconnected {
            return;
        }

        self.state = ConnectionState::Connecting;
        cx.notify();

        // For now, simulate connection
        // TODO: Implement actual TCP connection
        self.state = ConnectionState::Connected;
        cx.emit(CherryLinkEvent::Connected);
        cx.notify();
    }

    /// Disconnect from the CherryLink server
    pub fn disconnect(&mut self, cx: &mut Context<Self>) {
        self.state = ConnectionState::Disconnected;
        cx.emit(CherryLinkEvent::Disconnected);
        cx.notify();
    }

    /// Send a play start request
    pub fn play_start(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        // TODO: Send actual request
        self.play_state = PlayState::Playing;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play stop request
    pub fn play_stop(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        // TODO: Send actual request
        self.play_state = PlayState::Stopped;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play pause request
    pub fn play_pause(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        // TODO: Send actual request
        self.play_state = PlayState::Paused;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play resume request
    pub fn play_resume(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        // TODO: Send actual request
        self.play_state = PlayState::Playing;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a live coding build request
    pub fn build_live_coding(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        // TODO: Send actual request
        self.build_status = BuildStatus::Compiling;
        cx.emit(CherryLinkEvent::BuildStatusChanged(self.build_status));
        cx.notify();
    }
}

use cherry_link::{BuildConfiguration, CherryLinkConnection, PlayState};
use gpui::{
    div, prelude::*, px, App, Context, Corner, Entity, IntoElement, ParentElement,
    Render, Styled, Subscription, WeakEntity, Window,
};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use ui::{
    prelude::*, Button, ButtonStyle, ContextMenu, IconButton, IconName, PopoverMenu, Tooltip,
};
use workspace::Workspace;

/// PIE (Play-In-Editor) modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PieMode {
    #[default]
    SelectedViewport,
    MobilePreview,
    NewEditorWindow,
    StandaloneGame,
    VRPreview,
    Simulate,
}

impl PieMode {
    pub fn all() -> &'static [PieMode] {
        &[
            PieMode::SelectedViewport,
            PieMode::MobilePreview,
            PieMode::NewEditorWindow,
            PieMode::StandaloneGame,
            PieMode::VRPreview,
            PieMode::Simulate,
        ]
    }
}

impl std::fmt::Display for PieMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PieMode::SelectedViewport => write!(f, "Selected Viewport"),
            PieMode::MobilePreview => write!(f, "Mobile Preview"),
            PieMode::NewEditorWindow => write!(f, "New Editor Window"),
            PieMode::StandaloneGame => write!(f, "Standalone Game"),
            PieMode::VRPreview => write!(f, "VR Preview"),
            PieMode::Simulate => write!(f, "Simulate"),
        }
    }
}

/// Unreal Engine toolbar component
pub struct UnrealToolbar {
    workspace: WeakEntity<Workspace>,
    connection: Option<Entity<CherryLinkConnection>>,
    selected_config: BuildConfiguration,
    selected_pie_mode: PieMode,
    visible: bool,
    _subscriptions: Vec<Subscription>,
    /// Running UE process
    ue_process: Arc<Mutex<Option<Child>>>,
}

impl UnrealToolbar {
    pub fn new(workspace: WeakEntity<Workspace>, _cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            connection: None,
            selected_config: BuildConfiguration::default(),
            selected_pie_mode: PieMode::default(),
            visible: true,
            _subscriptions: Vec::new(),
            ue_process: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_connection(&mut self, connection: Entity<CherryLinkConnection>, cx: &mut Context<Self>) {
        self.connection = Some(connection.clone());
        self._subscriptions.push(cx.observe(&connection, |this, _, cx| {
            cx.notify();
        }));
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        cx.notify();
    }

    pub fn toggle_visible(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn selected_configuration(&self) -> BuildConfiguration {
        self.selected_config
    }

    pub fn set_selected_configuration(&mut self, config: BuildConfiguration, cx: &mut Context<Self>) {
        self.selected_config = config;
        cx.notify();
    }

    /// Get the engine path from global state
    fn engine_path(&self, cx: &App) -> Option<PathBuf> {
        cx.try_global::<cherry_link::UnrealProjectInfoGlobal>()
            .and_then(|g| g.engine_path())
    }

    /// Get the project path from global state
    fn project_path(&self, cx: &App) -> Option<PathBuf> {
        cx.try_global::<cherry_link::UnrealProjectInfoGlobal>()
            .and_then(|g| g.project_path())
    }

    /// Check if UE is currently running
    pub fn is_ue_running(&self) -> bool {
        if let Ok(guard) = self.ue_process.lock() {
            if guard.is_some() {
                return true;
            }
        }
        false
    }

    /// Get the path to the Unreal Editor executable
    fn get_editor_executable(&self, cx: &App) -> Option<PathBuf> {
        let engine_path = self.engine_path(cx)?;

        #[cfg(target_os = "windows")]
        {
            let exe_path = engine_path.join("Binaries/Win64/UnrealEditor.exe");
            if exe_path.exists() {
                return Some(exe_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let exe_path = engine_path.join("Binaries/Mac/UnrealEditor.app/Contents/MacOS/UnrealEditor");
            if exe_path.exists() {
                return Some(exe_path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let exe_path = engine_path.join("Binaries/Linux/UnrealEditor");
            if exe_path.exists() {
                return Some(exe_path);
            }
        }

        None
    }

    /// Launch Unreal Editor with the current project
    pub fn launch_unreal_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor_exe) = self.get_editor_executable(cx) else {
            log::warn!("Unreal Editor executable not found");
            return;
        };

        let mut cmd = Command::new(&editor_exe);

        // Add project path as argument if available
        if let Some(project_path) = self.project_path(cx) {
            cmd.arg(project_path);
        }

        match cmd.spawn() {
            Ok(child) => {
                log::info!("Launched Unreal Editor: {:?}", editor_exe);
                if let Ok(mut guard) = self.ue_process.lock() {
                    *guard = Some(child);
                }
                cx.notify();
            }
            Err(e) => {
                log::error!("Failed to launch Unreal Editor: {}", e);
            }
        }
    }

    /// Stop the running Unreal Editor process
    pub fn stop_unreal_editor(&mut self, cx: &mut Context<Self>) {
        if let Ok(mut guard) = self.ue_process.lock() {
            if let Some(ref mut child) = *guard {
                match child.kill() {
                    Ok(_) => {
                        log::info!("Stopped Unreal Editor process");
                    }
                    Err(e) => {
                        log::error!("Failed to stop Unreal Editor: {}", e);
                    }
                }
                *guard = None;
            }
        }
        cx.notify();
    }

    /// Check and update process status
    fn check_process_status(&mut self) {
        if let Ok(mut guard) = self.ue_process.lock() {
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        // Process has exited
                        *guard = None;
                    }
                    Ok(None) => {
                        // Process is still running
                    }
                    Err(e) => {
                        log::error!("Error checking process status: {}", e);
                        *guard = None;
                    }
                }
            }
        }
    }

    fn is_connected(&self, cx: &App) -> bool {
        self.connection
            .as_ref()
            .map(|c| c.read(cx).is_connected())
            .unwrap_or(false)
    }

    fn play_state(&self, cx: &App) -> PlayState {
        self.connection
            .as_ref()
            .map(|c| c.read(cx).play_state())
            .unwrap_or_default()
    }

    // ==================== PIE Section ====================

    fn render_pie_play_pause(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);
        let play_state = self.play_state(cx);

        let (icon, tooltip) = match play_state {
            PlayState::Playing | PlayState::Simulating => (IconName::DebugPause, "Pause PIE"),
            PlayState::Paused => (IconName::PlayFilled, "Resume PIE"),
            PlayState::Stopped => (IconName::PlayFilled, "Start PIE"),
        };

        IconButton::new("pie-play-pause", icon)
            .icon_size(IconSize::Small)
            .disabled(!is_connected)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(move |this, _, _window, cx| {
                if let Some(connection) = &this.connection {
                    connection.update(cx, |conn, cx| {
                        match play_state {
                            PlayState::Playing | PlayState::Simulating => conn.play_pause(cx),
                            PlayState::Paused => conn.play_resume(cx),
                            PlayState::Stopped => conn.play_start(cx),
                        }
                    });
                }
            }))
    }

    fn render_pie_stop(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);
        let play_state = self.play_state(cx);
        let is_playing = matches!(play_state, PlayState::Playing | PlayState::Simulating | PlayState::Paused);

        IconButton::new("pie-stop", IconName::Stop)
            .icon_size(IconSize::Small)
            .disabled(!is_connected || !is_playing)
            .tooltip(Tooltip::text("Stop PIE"))
            .on_click(cx.listener(|this, _, _window, cx| {
                if let Some(connection) = &this.connection {
                    connection.update(cx, |conn, cx| {
                        conn.play_stop(cx);
                    });
                }
            }))
    }

    fn render_pie_mode_dropdown(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_pie_mode;
        let this = cx.entity().downgrade();

        PopoverMenu::new("pie-mode-dropdown")
            .anchor(Corner::TopRight)
            .trigger(
                Button::new("pie-mode-trigger", selected.to_string())
                    .style(ButtonStyle::Subtle)
                    .icon(IconName::ChevronDown)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
            )
            .menu(move |window, cx| {
                let this = this.clone();
                Some(ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                    for &mode in PieMode::all() {
                        let is_selected = mode == selected;
                        let this = this.clone();
                        menu = menu.toggleable_entry(
                            mode.to_string(),
                            is_selected,
                            IconPosition::End,
                            None,
                            move |_window, cx| {
                                this.update(cx, |toolbar, cx| {
                                    toolbar.selected_pie_mode = mode;
                                    cx.notify();
                                }).ok();
                            },
                        );
                    }
                    menu
                }))
            })
    }

    // ==================== Config Section ====================

    fn render_config_dropdown(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_config;
        let this = cx.entity().downgrade();

        PopoverMenu::new("config-dropdown")
            .anchor(Corner::TopRight)
            .trigger(
                Button::new("config-trigger", selected.to_string())
                    .style(ButtonStyle::Subtle)
                    .icon(IconName::ChevronDown)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
            )
            .menu(move |window, cx| {
                let this = this.clone();
                Some(ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                    for &config in BuildConfiguration::all() {
                        let is_selected = config == selected;
                        let this = this.clone();
                        menu = menu.toggleable_entry(
                            config.to_string(),
                            is_selected,
                            IconPosition::End,
                            None,
                            move |_window, cx| {
                                this.update(cx, |toolbar, cx| {
                                    toolbar.selected_config = config;
                                    cx.notify();
                                }).ok();
                            },
                        );
                    }
                    menu
                }))
            })
    }

    // ==================== Launch Section ====================

    fn render_launch_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_engine = self.engine_path(cx).is_some();
        let is_running = self.is_ue_running();

        let tooltip = if !has_engine {
            "No engine path configured"
        } else if is_running {
            "Unreal Engine is already running"
        } else {
            "Launch Unreal Engine"
        };

        IconButton::new("launch", IconName::PlayFilled)
            .icon_size(IconSize::Small)
            .icon_color(if has_engine && !is_running { Color::Success } else { Color::Muted })
            .disabled(!has_engine || is_running)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(|this, _, _window, cx| {
                this.launch_unreal_editor(cx);
            }))
    }

    fn render_launch_debug_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_engine = self.engine_path(cx).is_some();
        let is_running = self.is_ue_running();

        IconButton::new("launch-debug", IconName::Debug)
            .icon_size(IconSize::Small)
            .disabled(!has_engine || is_running)
            .tooltip(Tooltip::text("Launch with Debugger (not yet implemented)"))
            .on_click(cx.listener(|_this, _, _window, _cx| {
                // TODO: Launch UE with debugger attached
            }))
    }

    fn render_stop_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.is_ue_running();

        IconButton::new("stop-ue", IconName::Stop)
            .icon_size(IconSize::Small)
            .icon_color(if is_running { Color::Error } else { Color::Muted })
            .disabled(!is_running)
            .tooltip(Tooltip::text("Stop Unreal Engine"))
            .on_click(cx.listener(|this, _, _window, cx| {
                this.stop_unreal_editor(cx);
            }))
    }

    // ==================== Status Section ====================

    fn render_connection_status(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);
        let (icon, color, tooltip) = if is_connected {
            (IconName::Check, Color::Success, "Connected to Unreal Engine")
        } else {
            (IconName::XCircle, Color::Error, "Disconnected from Unreal Engine")
        };

        IconButton::new("connection-status", icon)
            .icon_size(IconSize::Small)
            .icon_color(color)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(|this, _, _window, cx| {
                if let Some(connection) = &this.connection {
                    let is_connected = connection.read(cx).is_connected();
                    connection.update(cx, |conn, cx| {
                        if is_connected {
                            conn.disconnect(cx);
                        } else {
                            conn.connect(cx);
                        }
                    });
                }
            }))
    }

    // ==================== Helpers ====================

    fn render_separator(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(16.0))
            .w(px(1.0))
            .bg(cx.theme().colors().border)
    }
}

impl Render for UnrealToolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        div()
            .id("unreal-toolbar")
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .bg(cx.theme().colors().toolbar_background)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            // Spacer to push everything to the right
            .child(div().flex_grow())
            // PIE Section: Play/Pause, Stop, Mode dropdown
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(self.render_pie_play_pause(window, cx))
                    .child(self.render_pie_stop(window, cx))
                    .child(self.render_pie_mode_dropdown(window, cx))
            )
            // Separator
            .child(self.render_separator(cx))
            // Config Section: Build configuration dropdown
            .child(self.render_config_dropdown(window, cx))
            // Separator
            .child(self.render_separator(cx))
            // Launch Section: Launch, Debug, Stop
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(self.render_launch_button(window, cx))
                    .child(self.render_launch_debug_button(window, cx))
                    .child(self.render_stop_button(window, cx))
            )
            // Separator
            .child(self.render_separator(cx))
            // Status Section: Connection status
            .child(self.render_connection_status(window, cx))
            .into_any_element()
    }
}

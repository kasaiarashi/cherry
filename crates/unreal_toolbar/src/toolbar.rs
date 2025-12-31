use cherry_link::{CherryLinkConnection, PlayState};
use gpui::{
    div, prelude::*, px, App, Context, Corner, Entity, IntoElement, ParentElement,
    Render, Styled, Subscription, Task, WeakEntity, Window,
};
use futures::{channel::mpsc, StreamExt};
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use ui::{
    prelude::*, Button, ButtonStyle, ContextMenu, IconButton, IconName, PopoverMenu, Tooltip,
};
use unreal_panel::BuildPanel;
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
    selected_config: String,
    selected_platform: String,
    selected_pie_mode: PieMode,
    visible: bool,
    _subscriptions: Vec<Subscription>,
    /// Running build process flag
    is_building: Arc<Mutex<bool>>,
    /// Build output reader task
    _build_task: Option<Task<()>>,
}

impl UnrealToolbar {
    pub fn new(workspace: WeakEntity<Workspace>, _cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            connection: None,
            selected_config: "Development Editor".to_string(),
            selected_platform: "Win64".to_string(),
            selected_pie_mode: PieMode::default(),
            visible: true,
            _subscriptions: Vec::new(),
            is_building: Arc::new(Mutex::new(false)),
            _build_task: None,
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

    pub fn selected_configuration(&self) -> &str {
        &self.selected_config
    }

    pub fn selected_platform(&self) -> &str {
        &self.selected_platform
    }

    pub fn set_selected_configuration(&mut self, config: String, cx: &mut Context<Self>) {
        self.selected_config = config;
        cx.notify();
    }

    pub fn set_selected_platform(&mut self, platform: String, cx: &mut Context<Self>) {
        self.selected_platform = platform;
        cx.notify();
    }

    fn configurations(&self, cx: &App) -> Vec<String> {
        let configs = cx.try_global::<cherry_link::UnrealProjectInfoGlobal>()
            .map(|g| g.configurations())
            .unwrap_or_default();
        if configs.is_empty() {
            vec![
                "Development Editor".to_string(),
                "DebugGame Editor".to_string(),
                "Shipping".to_string(),
            ]
        } else {
            configs
        }
    }

    fn platforms(&self, cx: &App) -> Vec<String> {
        let platforms = cx.try_global::<cherry_link::UnrealProjectInfoGlobal>()
            .map(|g| g.platforms())
            .unwrap_or_default();
        if platforms.is_empty() {
            vec!["Win64".to_string()]
        } else {
            platforms
        }
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

    /// Check if a build is currently running
    pub fn is_building(&self) -> bool {
        self.is_building.lock().map(|g| *g).unwrap_or(false)
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

    /// Launch Unreal Editor with the current project (builds first)
    pub fn launch_unreal_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor_exe) = self.get_editor_executable(cx) else {
            log::warn!("Unreal Editor executable not found");
            return;
        };

        let Some(project_path) = self.project_path(cx) else {
            log::warn!("Project path not found");
            return;
        };

        log::info!("Starting build before launching Unreal Editor");

        let editor_exe_clone = editor_exe.clone();
        let project_path_clone = project_path.clone();

        // First, run the build
        self.start_build_internal(cx, move |success, _cx| {
            if !success {
                log::error!("Build failed, not launching Unreal Editor");
                return;
            }

            log::info!("Build succeeded, launching Unreal Editor");

            // Launch Unreal Editor with -skipcompile flag
            let mut cmd = Command::new(&editor_exe_clone);
            cmd.arg(&project_path_clone);
            cmd.arg("-skipcompile");

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                const DETACHED_PROCESS: u32 = 0x00000008;
                cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
            }

            match cmd.spawn() {
                Ok(_child) => {
                    log::info!("Launched Unreal Editor: {:?}", editor_exe_clone);
                }
                Err(e) => {
                    log::error!("Failed to launch Unreal Editor: {}", e);
                }
            }
        });
    }

    /// Get the project name from the .uproject file
    fn project_name(&self, cx: &App) -> Option<String> {
        cx.try_global::<cherry_link::UnrealProjectInfoGlobal>()
            .and_then(|g| g.project_path())
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
    }

    /// Get architecture string from platform
    fn architecture_for_platform(&self, platform: &str) -> &'static str {
        match platform {
            "Win64" => "x64",
            "Linux" => "x64",
            "Mac" => "arm64",
            _ => "x64",
        }
    }

    /// Get the path to the Build.bat script
    fn get_build_script(&self, cx: &App) -> Option<PathBuf> {
        let engine_path = self.engine_path(cx)?;

        #[cfg(target_os = "windows")]
        {
            let script = engine_path.join("Build/BatchFiles/Build.bat");
            if script.exists() {
                return Some(script);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let script = engine_path.join("Build/BatchFiles/Linux/Build.sh");
            if script.exists() {
                return Some(script);
            }
            let script = engine_path.join("Build/BatchFiles/Mac/Build.sh");
            if script.exists() {
                return Some(script);
            }
        }

        None
    }

    /// Execute a build command with a callback
    fn start_build_internal<F>(&mut self, cx: &mut Context<Self>, on_complete: F)
    where
        F: FnOnce(bool, &mut Context<Self>) + 'static,
    {
        self.start_build_with_callback(cx, Some(Box::new(on_complete)));
    }

    /// Execute a build command
    pub fn start_build(&mut self, cx: &mut Context<Self>) {
        self.start_build_with_callback(cx, None);
    }

    /// Internal build implementation with optional callback
    fn start_build_with_callback(
        &mut self,
        cx: &mut Context<Self>,
        on_complete: Option<Box<dyn FnOnce(bool, &mut Context<Self>) + 'static>>,
    ) {
        if self.is_building() {
            log::warn!("Build already in progress");
            return;
        }

        let Some(build_script) = self.get_build_script(cx) else {
            log::warn!("Build script not found");
            return;
        };

        let Some(project_name) = self.project_name(cx) else {
            log::warn!("Project name not found");
            return;
        };

        let Some(project_path) = self.project_path(cx) else {
            log::warn!("Project path not found");
            return;
        };

        let platform = self.selected_platform.clone();
        let config = self.selected_config.clone();
        let arch = self.architecture_for_platform(&platform).to_string();

        // Parse configuration: "Development Editor" -> target suffix "Editor", config "Development"
        // Configuration format from sln: "Development Editor", "DebugGame Editor", "Shipping", etc.
        let is_editor_build = config.contains("Editor");
        let base_config = config.replace(" Editor", "").replace("Editor", "");
        let base_config = base_config.trim().to_string();

        // Build target name: ProjectName + "Editor" suffix for editor builds
        let target_name = if is_editor_build {
            format!("{}Editor", project_name)
        } else {
            project_name.clone()
        };

        // Set building flag
        if let Ok(mut guard) = self.is_building.lock() {
            *guard = true;
        }

        let is_building = self.is_building.clone();
        let workspace = self.workspace.clone();

        log::info!("Starting build: {:?} {} {} {} -Project={} -WaitMutex -FromMsBuild -architecture={}",
            build_script, target_name, platform, base_config, project_path.display(), arch);

        // Create async channel for build output
        let (tx, mut rx) = mpsc::unbounded::<BuildMessage>();

        // Spawn background thread to run build and send output
        let tx_clone = tx.clone();
        thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let mut cmd = {
                // On Windows, call the batch file directly without cmd.exe wrapper
                // This ensures proper output capture
                let mut c = Command::new(&build_script);
                c.arg(&target_name);
                c.arg(&platform);
                c.arg(&base_config);
                c.arg(format!("-Project={}", project_path.display()));
                c.arg("-WaitMutex");
                c.arg("-FromMsBuild");
                c.arg(format!("-architecture={}", arch));

                // Ensure we don't create a new console window
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                c.creation_flags(CREATE_NO_WINDOW);

                c
            };

            #[cfg(not(target_os = "windows"))]
            let mut cmd = {
                let mut c = Command::new(&build_script);
                c.arg(&target_name);
                c.arg(&platform);
                c.arg(&base_config);
                c.arg(format!("-Project={}", project_path.display()));
                c.arg("-WaitMutex");
                c.arg("-FromMsBuild");
                c.arg(format!("-architecture={}", arch));
                c
            };

            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    log::info!("Build process spawned successfully");
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    // Read stdout and stderr in parallel using separate threads
                    let tx_stdout = tx_clone.clone();
                    let stdout_handle = stdout.map(|stdout| {
                        thread::spawn(move || {
                            log::info!("stdout reader thread started");
                            use std::io::Read;
                            let mut reader = BufReader::with_capacity(8192, stdout);
                            let mut line_count = 0;
                            let mut buffer = String::new();
                            let mut char_buffer = [0u8; 1];

                            // Read character by character to ensure we get output even if not line-buffered
                            while let Ok(1) = reader.read(&mut char_buffer) {
                                let ch = char_buffer[0] as char;
                                if ch == '\n' || ch == '\r' {
                                    if !buffer.is_empty() {
                                        line_count += 1;
                                        let line = buffer.trim_end().to_string();
                                        if !line.is_empty() {
                                            let _ = tx_stdout.unbounded_send(BuildMessage::Line(line));
                                        }
                                        buffer.clear();
                                    }
                                } else if ch != '\r' {  // Skip carriage returns
                                    buffer.push(ch);
                                }
                            }
                            // Send any remaining content
                            if !buffer.is_empty() {
                                line_count += 1;
                                let _ = tx_stdout.unbounded_send(BuildMessage::Line(buffer));
                            }
                            log::info!("stdout reader thread finished, read {} lines", line_count);
                        })
                    });

                    let tx_stderr = tx_clone.clone();
                    let stderr_handle = stderr.map(|stderr| {
                        thread::spawn(move || {
                            log::info!("stderr reader thread started");
                            use std::io::Read;
                            let mut reader = BufReader::with_capacity(8192, stderr);
                            let mut line_count = 0;
                            let mut buffer = String::new();
                            let mut char_buffer = [0u8; 1];

                            // Read character by character to ensure we get output even if not line-buffered
                            while let Ok(1) = reader.read(&mut char_buffer) {
                                let ch = char_buffer[0] as char;
                                if ch == '\n' || ch == '\r' {
                                    if !buffer.is_empty() {
                                        line_count += 1;
                                        let line = buffer.trim_end().to_string();
                                        if !line.is_empty() {
                                            let _ = tx_stderr.unbounded_send(BuildMessage::Line(line));
                                        }
                                        buffer.clear();
                                    }
                                } else if ch != '\r' {  // Skip carriage returns
                                    buffer.push(ch);
                                }
                            }
                            // Send any remaining content
                            if !buffer.is_empty() {
                                line_count += 1;
                                let _ = tx_stderr.unbounded_send(BuildMessage::Line(buffer));
                            }
                            log::info!("stderr reader thread finished, read {} lines", line_count);
                        })
                    });

                    // Wait for both reader threads
                    if let Some(handle) = stdout_handle {
                        let _ = handle.join();
                    }
                    if let Some(handle) = stderr_handle {
                        let _ = handle.join();
                    }

                    // Wait for process
                    let success = child.wait().map(|s| s.success()).unwrap_or(false);
                    let _ = tx_clone.unbounded_send(BuildMessage::Finished(success));
                }
                Err(e) => {
                    let _ = tx_clone.unbounded_send(BuildMessage::Line(format!("Failed to start build: {}", e)));
                    let _ = tx_clone.unbounded_send(BuildMessage::Finished(false));
                }
            }
        });

        // Spawn foreground task to receive messages and update panel
        let this_entity = cx.entity().downgrade();
        let on_complete = Arc::new(Mutex::new(on_complete));
        self._build_task = Some(cx.spawn(async move |_this, cx| {
            // Notify build panel that build started
            log::info!("Build task started, notifying panel");
            let panel_result = cx.update(|cx| {
                workspace.update(cx, |workspace, cx| {
                    if let Some(panel) = workspace.panel::<BuildPanel>(cx) {
                        log::info!("BuildPanel found, starting build");
                        panel.update(cx, |panel: &mut BuildPanel, cx| {
                            panel.start_build(cx);
                            // Add a test message to verify panel is working
                            panel.add_line("Build process initializing...".to_string(), cx);
                        });
                        true
                    } else {
                        log::warn!("BuildPanel not found in workspace");
                        false
                    }
                })
            });
            if let Err(e) = panel_result {
                log::error!("Failed to start build panel: {:?}", e);
            } else {
                log::info!("Build panel start_build completed successfully");
            }

            // Process messages from build thread (async stream)
            log::info!("Starting message receive loop");
            while let Some(message) = rx.next().await {
                match message {
                    BuildMessage::Line(line) => {
                        log::info!("Build output: {}", line);
                        let workspace = workspace.clone();
                        let update_result = cx.update(|cx| {
                            workspace.update(cx, |workspace, cx| {
                                if let Some(panel) = workspace.panel::<BuildPanel>(cx) {
                                    panel.update(cx, |panel: &mut BuildPanel, cx| {
                                        panel.add_line(line, cx);
                                    });
                                    true
                                } else {
                                    log::warn!("BuildPanel not found when adding line");
                                    false
                                }
                            })
                        });
                        if let Err(e) = update_result {
                            log::error!("Failed to update panel with line: {:?}", e);
                            break;
                        }
                    }
                    BuildMessage::Finished(success) => {
                        // Clear building flag
                        if let Ok(mut guard) = is_building.lock() {
                            *guard = false;
                        }

                        // Notify panel
                        let _ = cx.update(|cx| {
                            let _ = workspace.update(cx, |workspace, cx| {
                                if let Some(panel) = workspace.panel::<BuildPanel>(cx) {
                                    panel.update(cx, |panel: &mut BuildPanel, cx| {
                                        panel.finish_build(success, cx);
                                    });
                                }
                            });
                        });

                        // Call the completion callback if provided
                        if let Ok(mut callback_opt) = on_complete.lock() {
                            if let Some(callback) = callback_opt.take() {
                                log::info!("Build finished with success={}, calling callback", success);
                                let _ = this_entity.update(cx, |_this, cx| {
                                    callback(success, cx);
                                });
                            }
                        }
                        break;
                    }
                }
            }

            // Channel closed - clear building flag if not already cleared
            log::info!("Message receive loop finished");
            if let Ok(mut guard) = is_building.lock() {
                *guard = false;
            }
        }));

        cx.notify();
    }
}

/// Messages sent from build thread to UI
enum BuildMessage {
    Line(String),
    Finished(bool),
}

impl UnrealToolbar {

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
        let selected = self.selected_config.clone();
        let configs = self.configurations(cx);
        let this = cx.entity().downgrade();

        PopoverMenu::new("config-dropdown")
            .anchor(Corner::TopRight)
            .trigger(
                Button::new("config-trigger", selected.clone())
                    .style(ButtonStyle::Subtle)
                    .icon(IconName::ChevronDown)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
            )
            .menu(move |window, cx| {
                let this = this.clone();
                let selected = selected.clone();
                let configs = configs.clone();
                Some(ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                    for config in &configs {
                        let is_selected = config == &selected;
                        let this = this.clone();
                        let config = config.clone();
                        menu = menu.toggleable_entry(
                            config.clone(),
                            is_selected,
                            IconPosition::End,
                            None,
                            move |_window, cx| {
                                let config = config.clone();
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

    fn render_platform_dropdown(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_platform.clone();
        let platforms = self.platforms(cx);
        let this = cx.entity().downgrade();

        PopoverMenu::new("platform-dropdown")
            .anchor(Corner::TopRight)
            .trigger(
                Button::new("platform-trigger", selected.clone())
                    .style(ButtonStyle::Subtle)
                    .icon(IconName::ChevronDown)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
            )
            .menu(move |window, cx| {
                let this = this.clone();
                let selected = selected.clone();
                let platforms = platforms.clone();
                Some(ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                    for platform in &platforms {
                        let is_selected = platform == &selected;
                        let this = this.clone();
                        let platform = platform.clone();
                        menu = menu.toggleable_entry(
                            platform.clone(),
                            is_selected,
                            IconPosition::End,
                            None,
                            move |_window, cx| {
                                let platform = platform.clone();
                                this.update(cx, |toolbar, cx| {
                                    toolbar.selected_platform = platform;
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
        let is_running = self.is_building();

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

    fn render_build_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_engine = self.engine_path(cx).is_some();
        let has_project = self.project_path(cx).is_some();
        let is_running = self.is_building();

        let tooltip = if !has_engine {
            "No engine path configured"
        } else if !has_project {
            "No project path configured"
        } else if is_running {
            "Build/Process already running"
        } else {
            "Build Project"
        };

        let can_build = has_engine && has_project && !is_running;

        IconButton::new("build", IconName::ToolHammer)
            .icon_size(IconSize::Small)
            .icon_color(if can_build { Color::Default } else { Color::Muted })
            .disabled(!can_build)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(|this, _, _window, cx| {
                this.start_build(cx);
            }))
    }

    fn render_launch_debug_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_engine = self.engine_path(cx).is_some();
        let is_running = self.is_building();

        IconButton::new("launch-debug", IconName::Debug)
            .icon_size(IconSize::Small)
            .disabled(!has_engine || is_running)
            .tooltip(Tooltip::text("Launch with Debugger (not yet implemented)"))
            .on_click(cx.listener(|_this, _, _window, _cx| {
                // TODO: Launch UE with debugger attached
            }))
    }

    fn render_stop_button(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.is_building();

        IconButton::new("stop-ue", IconName::Stop)
            .icon_size(IconSize::Small)
            .icon_color(if is_running { Color::Error } else { Color::Muted })
            .disabled(!is_running)
            .tooltip(Tooltip::text("Stop Unreal Engine (not implemented)"))
            .on_click(cx.listener(|_this, _, _window, _cx| {
                // TODO: Implement stopping UE process
                log::info!("Stop UE not implemented");
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
            // Config Section: Build configuration and platform dropdowns
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(self.render_config_dropdown(window, cx))
                    .child(self.render_platform_dropdown(window, cx))
            )
            // Separator
            .child(self.render_separator(cx))
            // Build Section
            .child(self.render_build_button(window, cx))
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

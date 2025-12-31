use crate::ToggleFocus;
use anyhow::Result;
use cherry_link::{CherryLinkConnection, CherryLinkEvent, LogMessage};
use collections::VecDeque;
use editor::{Editor, EditorMode, MultiBuffer, MultiBufferOffset, SizingBehavior};
use gpui::{
    Action, App, AsyncWindowContext, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, HighlightStyle, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, SharedString, Styled, Subscription, UniformListScrollHandle, WeakEntity, Window, div,
    prelude::*, px, rgb,
};
use language::Buffer;
use ui::{Label, h_flex, prelude::*, v_flex};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

const UNREAL_PANEL_KEY: &str = "UnrealPanel";
const MAX_LOG_ENTRIES: usize = 10000;

// Marker types for log highlighting by verbosity
struct ErrorLogHighlight;
struct WarningLogHighlight;
struct DisplayLogHighlight;
struct NormalLogHighlight;

pub struct UnrealPanel {
    workspace: WeakEntity<Workspace>,
    connection: Option<Entity<CherryLinkConnection>>,
    logs: VecDeque<LogEntry>,
    focus_handle: FocusHandle,
    width: Option<Pixels>,
    height: Option<Pixels>,
    auto_scroll: bool,
    scroll_handle: UniformListScrollHandle,
    log_buffer: Entity<Buffer>,
    log_editor: Option<Entity<Editor>>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct LogEntry {
    category: SharedString,
    verbosity: LogVerbosity,
    message: SharedString,
    start_offset: usize,
    end_offset: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogVerbosity {
    Log,
    Warning,
    Error,
    Display,
}

impl LogVerbosity {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "warning" => Self::Warning,
            "error" | "fatal" => Self::Error,
            "display" => Self::Display,
            _ => Self::Log,
        }
    }

    fn color(&self) -> ui::Color {
        match self {
            Self::Log => ui::Color::Muted,
            Self::Warning => ui::Color::Warning,
            Self::Error => ui::Color::Error,
            Self::Display => ui::Color::Accent,
        }
    }

    fn text_color(&self) -> Hsla {
        match self {
            Self::Log => rgb(0xFFFFFF).into(),        // White
            Self::Warning => rgb(0xFFD700).into(),    // Gold/Yellow
            Self::Error => rgb(0xFF4444).into(),      // Red
            Self::Display => rgb(0xFFFFFF).into(),    // White
        }
    }

    fn icon(&self) -> Option<IconName> {
        match self {
            Self::Warning => Some(IconName::Warning),
            Self::Error => Some(IconName::XCircle),
            Self::Display => Some(IconName::Info),
            _ => None,
        }
    }
}

impl UnrealPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, _window, cx| {
            cx.new(|cx| UnrealPanel::new(workspace.weak_handle(), cx))
        })
    }

    pub fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        log::info!("UnrealPanel::new - Creating new UnrealPanel");

        // Create a buffer for logs
        let log_buffer = cx.new(|cx| Buffer::local("", cx));

        Self {
            workspace,
            connection: None,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            focus_handle: cx.focus_handle(),
            width: None,
            height: Some(px(300.0)),
            auto_scroll: true,
            scroll_handle: UniformListScrollHandle::new(),
            log_buffer,
            log_editor: None,
            _subscriptions: Vec::new(),
        }
    }

    pub fn set_connection(
        &mut self,
        connection: Entity<CherryLinkConnection>,
        cx: &mut Context<Self>,
    ) {
        log::info!("UnrealPanel::set_connection called");

        let is_connected = connection.read(cx).is_connected();
        let conn_state = connection.read(cx).connection_state();
        log::info!(
            "UnrealPanel::set_connection - is_connected: {}, state: {:?}",
            is_connected,
            conn_state
        );

        self.connection = Some(connection.clone());
        self._subscriptions
            .push(cx.observe(&connection, |_this, _, cx| {
                cx.notify();
            }));
        // Subscribe to connection events
        self._subscriptions.push(cx.subscribe(
            &connection,
            |this, _, event: &CherryLinkEvent, cx| {
                match event {
                    CherryLinkEvent::LogReceived(log) => {
                        log::info!(
                            "UnrealPanel: Received log event - {} - {}",
                            log.category,
                            log.message
                        );
                        this.add_log(log.clone(), cx);
                        log::info!("UnrealPanel: Total logs now: {}", this.logs.len());
                    }
                    CherryLinkEvent::Connected => {
                        log::info!("UnrealPanel: Connected event received, adding test log");
                        // Add a test log to verify panel is working
                        let test_log = LogMessage {
                            category: "CherryLink".to_string(),
                            verbosity: "Display".to_string(),
                            message: "Successfully connected to Unreal Engine!".to_string(),
                            timestamp: String::new(),
                            frame: 0,
                            source_file: None,
                            source_line: None,
                        };
                        this.add_log(test_log, cx);

                        // Auto-focus the Unreal panel on connection
                        cx.emit(PanelEvent::Activate);
                    }
                    _ => {}
                }
            },
        ));

        // Note: Logging is now automatic in CherryLink, no subscription needed

        // // Add an immediate test log
        // log::info!("UnrealPanel: Adding immediate test log");
        // let test_log = LogMessage {
        //     category: "CherryLink".to_string(),
        //     verbosity: "Display".to_string(),
        //     message: "Panel initialized and connection set!".to_string(),
        //     timestamp: String::new(),
        //     frame: 0,
        //     source_file: None,
        //     source_line: None,
        // };
        // self.add_log(test_log, cx);

        cx.notify();
    }

    pub fn add_log(&mut self, log: LogMessage, cx: &mut Context<Self>) {
        log::info!(
            "UnrealPanel::add_log called - category: {}, message: {}",
            log.category,
            log.message
        );

        let verbosity = LogVerbosity::from_str(&log.verbosity);

        // Append to buffer with log type and consistent spacing (no brackets)
        let log_type = match verbosity {
            LogVerbosity::Error => "Error  ",     // 7 chars
            LogVerbosity::Warning => "Warning",   // 7 chars
            LogVerbosity::Display => "Display",   // 7 chars
            LogVerbosity::Log => "Log    ",       // 7 chars
        };

        // Format with fixed-width category (truncate or pad to 25 chars)
        let category = if log.category.len() > 25 {
            format!("{}...", &log.category[..22])
        } else {
            format!("{:25}", log.category)
        };

        let log_line = format!("{}  {}  {}\n", log_type, category, log.message);

        // Get the current buffer length before adding
        let (start_offset, end_offset) = self.log_buffer.update(cx, |buffer, cx| {
            let start = buffer.len();
            buffer.edit([(start..start, log_line.clone())], None, cx);
            let end = buffer.len() - 1; // Don't include the newline in highlight
            (start, end)
        });

        let entry = LogEntry {
            category: log.category.clone().into(),
            verbosity,
            message: log.message.clone().into(),
            start_offset,
            end_offset,
        };

        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);

        // Re-apply all highlights if editor exists
        if let Some(editor) = self.log_editor.clone() {
            self.update_highlights(&editor, cx);
        }

        log::info!(
            "UnrealPanel::add_log - logs count after push: {}",
            self.logs.len()
        );

        cx.notify();
        log::info!("UnrealPanel::add_log - called cx.notify()");
    }

    fn update_highlights(&mut self, editor: &Entity<Editor>, cx: &mut Context<Self>) {
        editor.update(cx, |editor, cx| {
            let buffer = editor.buffer().read(cx);
            let snapshot = buffer.snapshot(cx);

            // Group logs by verbosity
            let mut error_ranges = Vec::new();
            let mut warning_ranges = Vec::new();
            let mut display_ranges = Vec::new();
            let mut log_ranges = Vec::new();

            for entry in &self.logs {
                let start_anchor = snapshot.anchor_before(MultiBufferOffset(entry.start_offset));
                let end_anchor = snapshot.anchor_after(MultiBufferOffset(entry.end_offset));

                match entry.verbosity {
                    LogVerbosity::Error => error_ranges.push(start_anchor..end_anchor),
                    LogVerbosity::Warning => warning_ranges.push(start_anchor..end_anchor),
                    LogVerbosity::Display => display_ranges.push(start_anchor..end_anchor),
                    LogVerbosity::Log => log_ranges.push(start_anchor..end_anchor),
                }
            }

            // Apply highlights for each verbosity type
            if !error_ranges.is_empty() {
                editor.highlight_text::<ErrorLogHighlight>(
                    error_ranges,
                    HighlightStyle {
                        color: Some(rgb(0xFF4444).into()),
                        background_color: None,
                        font_weight: None,
                        font_style: None,
                        underline: None,
                        strikethrough: None,
                        fade_out: None,
                    },
                    cx,
                );
            }

            if !warning_ranges.is_empty() {
                editor.highlight_text::<WarningLogHighlight>(
                    warning_ranges,
                    HighlightStyle {
                        color: Some(rgb(0xFFD700).into()),
                        background_color: None,
                        font_weight: None,
                        font_style: None,
                        underline: None,
                        strikethrough: None,
                        fade_out: None,
                    },
                    cx,
                );
            }

            if !display_ranges.is_empty() {
                editor.highlight_text::<DisplayLogHighlight>(
                    display_ranges,
                    HighlightStyle {
                        color: Some(rgb(0xFFFFFF).into()),
                        background_color: None,
                        font_weight: None,
                        font_style: None,
                        underline: None,
                        strikethrough: None,
                        fade_out: None,
                    },
                    cx,
                );
            }

            if !log_ranges.is_empty() {
                editor.highlight_text::<NormalLogHighlight>(
                    log_ranges,
                    HighlightStyle {
                        color: Some(rgb(0xFFFFFF).into()),
                        background_color: None,
                        font_weight: None,
                        font_style: None,
                        underline: None,
                        strikethrough: None,
                        fade_out: None,
                    },
                    cx,
                );
            }
        });
    }

    pub fn clear_logs(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        self.log_buffer.update(cx, |buffer, cx| {
            let len = buffer.len();
            buffer.edit([(0..len, "")], None, cx);
        });

        // Clear highlights from editor
        if let Some(editor) = &self.log_editor {
            editor.update(cx, |editor, cx| {
                editor.highlight_text::<ErrorLogHighlight>(vec![], HighlightStyle::default(), cx);
                editor.highlight_text::<WarningLogHighlight>(vec![], HighlightStyle::default(), cx);
                editor.highlight_text::<DisplayLogHighlight>(vec![], HighlightStyle::default(), cx);
                editor.highlight_text::<NormalLogHighlight>(vec![], HighlightStyle::default(), cx);
            });
        }

        cx.notify();
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self
            .connection
            .as_ref()
            .map(|c| c.read(cx).is_connected())
            .unwrap_or(false);

        h_flex()
            .w_full()
            .justify_between()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new("Unreal Output").size(LabelSize::Small))
                    .child(
                        Label::new(if is_connected {
                            "Connected"
                        } else {
                            "Disconnected"
                        })
                        .size(LabelSize::XSmall)
                        .color(if is_connected {
                            ui::Color::Success
                        } else {
                            ui::Color::Error
                        }),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        ui::IconButton::new("copy-logs", IconName::Copy)
                            .icon_size(IconSize::Small)
                            .tooltip(ui::Tooltip::text("Copy All Logs to Clipboard"))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                let all_logs = this
                                    .logs
                                    .iter()
                                    .map(|entry| format!("[{}] {}", entry.category, entry.message))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                cx.write_to_clipboard(ClipboardItem::new_string(all_logs));
                            })),
                    )
                    .child(
                        ui::IconButton::new("clear-logs", IconName::Trash)
                            .icon_size(IconSize::Small)
                            .tooltip(ui::Tooltip::text("Clear Logs"))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.clear_logs(cx);
                            })),
                    )
                    .child(
                        ui::IconButton::new("toggle-autoscroll", IconName::ArrowDown)
                            .icon_size(IconSize::Small)
                            .icon_color(if self.auto_scroll {
                                ui::Color::Accent
                            } else {
                                ui::Color::Default
                            })
                            .tooltip(ui::Tooltip::text(if self.auto_scroll {
                                "Disable Auto-scroll"
                            } else {
                                "Enable Auto-scroll"
                            }))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.auto_scroll = !this.auto_scroll;
                                cx.notify();
                            })),
                    ),
            )
    }
}

impl EventEmitter<PanelEvent> for UnrealPanel {}

impl Focusable for UnrealPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for UnrealPanel {
    fn persistent_name() -> &'static str {
        "Unreal Panel"
    }

    fn panel_key() -> &'static str {
        UNREAL_PANEL_KEY
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Bottom | DockPosition::Left | DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn size(&self, _window: &Window, _cx: &App) -> Pixels {
        self.height.unwrap_or(px(300.0))
    }

    fn set_size(&mut self, size: Option<Pixels>, _window: &mut Window, cx: &mut Context<Self>) {
        self.height = size;
        cx.notify();
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::Terminal)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Unreal Output")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        3
    }
}

impl Render for UnrealPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        log::info!(
            "UnrealPanel::render called - log_count: {}",
            self.logs.len()
        );

        // Create editor on first render
        if self.log_editor.is_none() {
            let log_buffer = self.log_buffer.clone();
            let editor = cx.new(|cx| {
                let multibuffer = cx.new(|cx| MultiBuffer::singleton(log_buffer, cx));
                let mut editor = Editor::new(
                    EditorMode::Full {
                        scale_ui_elements_with_buffer_font_size: false,
                        show_active_line_background: false,
                        sizing_behavior: SizingBehavior::Default,
                    },
                    multibuffer,
                    None,
                    window,
                    cx,
                );
                editor.set_read_only(true);
                editor
            });
            self.log_editor = Some(editor);
        }

        v_flex()
            .id("unreal-panel")
            .key_context("UnrealPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().panel_background)
            .child(self.render_toolbar(window, cx))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .child(self.log_editor.as_ref().unwrap().clone()),
            )
    }
}

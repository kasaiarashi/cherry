use crate::ToggleFocus;
use anyhow::Result;
use cherry_link::{CherryLinkConnection, CherryLinkEvent, LogMessage};
use collections::VecDeque;
use gpui::{
    div, prelude::*, px, uniform_list, Action, App, AsyncWindowContext, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, ScrollStrategy, SharedString, Styled, Subscription, UniformListScrollHandle,
    WeakEntity, Window,
};
use ui::{h_flex, prelude::*, v_flex, Icon, IconName, Label};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

const UNREAL_PANEL_KEY: &str = "UnrealPanel";
const MAX_LOG_ENTRIES: usize = 10000;

pub struct UnrealPanel {
    workspace: WeakEntity<Workspace>,
    connection: Option<Entity<CherryLinkConnection>>,
    logs: VecDeque<LogEntry>,
    focus_handle: FocusHandle,
    width: Option<Pixels>,
    height: Option<Pixels>,
    auto_scroll: bool,
    scroll_handle: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct LogEntry {
    category: SharedString,
    verbosity: LogVerbosity,
    message: SharedString,
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
            Self::Log => ui::Color::Default,
            Self::Warning => ui::Color::Warning,
            Self::Error => ui::Color::Error,
            Self::Display => ui::Color::Info,
        }
    }

    fn icon(&self) -> Option<IconName> {
        match self {
            Self::Warning => Some(IconName::Warning),
            Self::Error => Some(IconName::XCircle),
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
        Self {
            workspace,
            connection: None,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            focus_handle: cx.focus_handle(),
            width: None,
            height: Some(px(300.0)),
            auto_scroll: false,
            scroll_handle: UniformListScrollHandle::new(),
            _subscriptions: Vec::new(),
        }
    }

    pub fn set_connection(&mut self, connection: Entity<CherryLinkConnection>, cx: &mut Context<Self>) {
        self.connection = Some(connection.clone());
        self._subscriptions.push(cx.observe(&connection, |_this, _, cx| {
            cx.notify();
        }));
        // Subscribe to connection events
        self._subscriptions.push(cx.subscribe(&connection, |this, _, event: &CherryLinkEvent, cx| {
            match event {
                CherryLinkEvent::LogReceived(log) => {
                    log::info!("UnrealPanel: Received log event - {} - {}", log.category, log.message);
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
                }
                _ => {}
            }
        }));

        // Subscribe to logging when connected
        connection.update(cx, |conn, cx| {
            conn.subscribe_logging(cx);
        });

        // Add an immediate test log
        log::info!("UnrealPanel: Adding immediate test log");
        let test_log = LogMessage {
            category: "CherryLink".to_string(),
            verbosity: "Display".to_string(),
            message: "Panel initialized and connection set!".to_string(),
            timestamp: String::new(),
            frame: 0,
            source_file: None,
            source_line: None,
        };
        self.add_log(test_log, cx);

        cx.notify();
    }

    pub fn add_log(&mut self, log: LogMessage, cx: &mut Context<Self>) {
        log::info!("UnrealPanel::add_log called - category: {}, message: {}", log.category, log.message);

        let entry = LogEntry {
            category: log.category.into(),
            verbosity: LogVerbosity::from_str(&log.verbosity),
            message: log.message.into(),
        };

        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);

        log::info!("UnrealPanel::add_log - logs count after push: {}", self.logs.len());

        // Auto-scroll to bottom when enabled
        if self.auto_scroll && !self.logs.is_empty() {
            self.scroll_handle.scroll_to_item(self.logs.len() - 1, ScrollStrategy::Bottom);
        }

        cx.notify();
        log::info!("UnrealPanel::add_log - called cx.notify()");
    }

    pub fn clear_logs(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        cx.notify();
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.connection
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
                        Label::new(if is_connected { "Connected" } else { "Disconnected" })
                            .size(LabelSize::XSmall)
                            .color(if is_connected { ui::Color::Success } else { ui::Color::Error }),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
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
                            .icon_color(if self.auto_scroll { ui::Color::Accent } else { ui::Color::Default })
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
        matches!(position, DockPosition::Bottom | DockPosition::Left | DockPosition::Right)
    }

    fn set_position(&mut self, _position: DockPosition, _window: &mut Window, cx: &mut Context<Self>) {
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
        let log_count = self.logs.len();
        log::info!("UnrealPanel::render called - log_count: {}", log_count);
        let logs: Vec<_> = self.logs.iter().cloned().collect();
        log::info!("UnrealPanel::render - logs vec size: {}", logs.len());
        let theme_colors = cx.theme().colors().clone();

        v_flex()
            .id("unreal-panel")
            .key_context("UnrealPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().panel_background)
            .child(self.render_toolbar(window, cx))
            .child(
                div()
                    .flex_grow()
                    .child(
                        div()
                            .p_2()
                            .bg(cx.theme().colors().elevated_surface_background)
                            .child(Label::new(format!("Log Count: {} (Test Div)", log_count)))
                    )
                    .child(
                        uniform_list(
                            "log-list",
                            log_count,
                            move |range, _window, _cx| {
                                log::info!("UnrealPanel::render - uniform_list closure called with range: {:?}", range);
                                log::info!("UnrealPanel::render - logs.len() in closure: {}", logs.len());
                                let theme = theme_colors.clone();
                                let items: Vec<_> = range
                                    .filter_map(|ix| {
                                        log::info!("UnrealPanel::render - trying to get log at index: {}", ix);
                                        logs.get(ix).cloned()
                                    })
                                    .map(|entry| {
                                        log::info!("UnrealPanel::render - rendering log entry: {} - {}", entry.category, entry.message);
                                        let verbosity_color = entry.verbosity.color();
                                        let icon = entry.verbosity.icon();

                                        h_flex()
                                            .w_full()
                                            .h(px(24.0))
                                            .gap_2()
                                            .px_2()
                                            .py_1()
                                            .bg(theme.ghost_element_background)
                                            .hover(|style| style.bg(theme.ghost_element_hover))
                                            .when_some(icon, |this, icon| {
                                                this.child(
                                                    Icon::new(icon)
                                                        .size(IconSize::Small)
                                                        .color(verbosity_color),
                                                )
                                            })
                                            .child(
                                                Label::new(entry.category.clone())
                                                    .size(LabelSize::Small)
                                                    .color(ui::Color::Muted),
                                            )
                                            .child(
                                                Label::new(entry.message.clone())
                                                    .size(LabelSize::Small)
                                                    .color(verbosity_color),
                                            )
                                    })
                                    .collect();
                                log::info!("UnrealPanel::render - collected {} items for rendering", items.len());
                                items
                            },
                        )
                        .flex_grow()
                        .track_scroll(&self.scroll_handle),
                    ),
            )
    }
}

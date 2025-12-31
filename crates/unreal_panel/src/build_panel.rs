use anyhow::Result;
use collections::VecDeque;
use gpui::{
    div, prelude::*, px, uniform_list, Action, App, AsyncWindowContext, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, ScrollStrategy, SharedString, Styled, UniformListScrollHandle, WeakEntity, Window,
};
use ui::{h_flex, prelude::*, v_flex, Icon, IconName, Label};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};
use log;

use crate::ToggleBuildPanel;

const BUILD_PANEL_KEY: &str = "BuildPanel";
const MAX_LOG_ENTRIES: usize = 50000;

pub struct BuildPanel {
    workspace: WeakEntity<Workspace>,
    logs: VecDeque<BuildLogEntry>,
    focus_handle: FocusHandle,
    width: Option<Pixels>,
    height: Option<Pixels>,
    auto_scroll: bool,
    scroll_handle: UniformListScrollHandle,
    is_building: bool,
}

#[derive(Clone)]
struct BuildLogEntry {
    line: SharedString,
    entry_type: BuildLogType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuildLogType {
    Info,
    Warning,
    Error,
    Success,
}

impl BuildLogType {
    fn from_line(line: &str) -> Self {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("failed") {
            Self::Error
        } else if lower.contains("warning") {
            Self::Warning
        } else if lower.contains("succeeded") || lower.contains("success") || lower.contains("complete") {
            Self::Success
        } else {
            Self::Info
        }
    }

    fn color(&self) -> ui::Color {
        match self {
            Self::Info => ui::Color::Default,
            Self::Warning => ui::Color::Warning,
            Self::Error => ui::Color::Error,
            Self::Success => ui::Color::Success,
        }
    }

    fn icon(&self) -> Option<IconName> {
        match self {
            Self::Warning => Some(IconName::Warning),
            Self::Error => Some(IconName::XCircle),
            Self::Success => Some(IconName::Check),
            Self::Info => None,
        }
    }
}

impl BuildPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, _window, cx| {
            cx.new(|cx| BuildPanel::new(workspace.weak_handle(), cx))
        })
    }

    pub fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            focus_handle: cx.focus_handle(),
            width: None,
            height: Some(px(300.0)),
            auto_scroll: true,
            scroll_handle: UniformListScrollHandle::new(),
            is_building: false,
        }
    }

    pub fn add_line(&mut self, line: String, cx: &mut Context<Self>) {
        log::debug!("BuildPanel::add_line called with: {}", line);
        let entry = BuildLogEntry {
            entry_type: BuildLogType::from_line(&line),
            line: line.into(),
        };

        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
        log::debug!("BuildPanel now has {} log entries", self.logs.len());

        if self.auto_scroll && !self.logs.is_empty() {
            self.scroll_handle.scroll_to_item(self.logs.len() - 1, ScrollStrategy::Bottom);
        }

        cx.notify();
    }

    pub fn start_build(&mut self, cx: &mut Context<Self>) {
        log::info!("BuildPanel::start_build called");
        self.logs.clear();
        self.is_building = true;
        self.add_line("=== Build Started ===".to_string(), cx);
        cx.emit(PanelEvent::Activate);
        log::info!("BuildPanel activated");
    }

    pub fn finish_build(&mut self, success: bool, cx: &mut Context<Self>) {
        self.is_building = false;
        if success {
            self.add_line("=== Build Succeeded ===".to_string(), cx);
        } else {
            self.add_line("=== Build Failed ===".to_string(), cx);
        }
        cx.notify();
    }

    pub fn clear_logs(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        cx.notify();
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(Label::new("Build Output").size(LabelSize::Small))
                    .when(self.is_building, |this| {
                        this.child(
                            Label::new("Building...")
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Warning),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        ui::IconButton::new("clear-build-logs", IconName::Trash)
                            .icon_size(IconSize::Small)
                            .tooltip(ui::Tooltip::text("Clear Logs"))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.clear_logs(cx);
                            })),
                    )
                    .child(
                        ui::IconButton::new("toggle-build-autoscroll", IconName::ArrowDown)
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

impl EventEmitter<PanelEvent> for BuildPanel {}

impl Focusable for BuildPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for BuildPanel {
    fn persistent_name() -> &'static str {
        "Build Panel"
    }

    fn panel_key() -> &'static str {
        BUILD_PANEL_KEY
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
        Some(IconName::ToolHammer)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Build Output")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleBuildPanel)
    }

    fn activation_priority(&self) -> u32 {
        4
    }
}

impl Render for BuildPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let log_count = self.logs.len();

        log::info!("BuildPanel::render called with {} logs", log_count);
        if log_count > 0 {
            log::info!("First log entry: {:?}", self.logs.front().map(|e| &e.line));
        }

        log::info!("Creating v_flex container");
        v_flex()
            .id("build-panel")
            .key_context("BuildPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().panel_background)
            .child(self.render_toolbar(window, cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child({
                        log::info!("Creating uniform_list with {} items", log_count);
                        uniform_list(
                            "build-log-list",
                            log_count,
                            cx.processor(|this: &mut BuildPanel, range, _window, _cx| {
                                log::info!("uniform_list callback invoked for range {:?}", range);
                                let mut items = Vec::new();
                                
                                for ix in range {
                                    if let Some(entry) = this.logs.get(ix) {
                                        log::trace!("Rendering log entry at index {}: {:?}", ix, &entry.line);
                                        let color = entry.entry_type.color();
                                        let icon = entry.entry_type.icon();

                                        items.push(
                                            h_flex()
                                                .id(ix)
                                                .w_full()
                                                .gap_2()
                                                .px_2()
                                                .py_0p5()
                                                .when_some(icon, |this, icon| {
                                                    this.child(
                                                        Icon::new(icon)
                                                            .size(IconSize::Small)
                                                            .color(color),
                                                    )
                                                })
                                                .child(
                                                    Label::new(entry.line.clone())
                                                        .size(LabelSize::Small)
                                                        .color(color),
                                                ),
                                        );
                                    }
                                }
                                
                                log::info!("Rendered {} log items for range", items.len());
                                items
                            }),
                        )
                        .h_full()
                        .track_scroll(&self.scroll_handle)
                    }),
            )
    }
}

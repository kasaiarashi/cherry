use anyhow::Result;
use collections::VecDeque;
use editor::{Editor, EditorMode, MultiBuffer, SizingBehavior};
use gpui::{
    div, prelude::*, px, uniform_list, Action, App, AsyncWindowContext, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, ScrollStrategy, SharedString, Styled, UniformListScrollHandle, WeakEntity, Window,
};
use language::Buffer;
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
    log_buffer: Entity<Buffer>,
    log_editor: Option<Entity<Editor>>,
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
        // Create a buffer for build logs
        let log_buffer = cx.new(|cx| Buffer::local("", cx));

        Self {
            workspace,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            focus_handle: cx.focus_handle(),
            width: None,
            height: Some(px(300.0)),
            auto_scroll: true,
            scroll_handle: UniformListScrollHandle::new(),
            is_building: false,
            log_buffer,
            log_editor: None,
        }
    }

    pub fn add_line(&mut self, line: String, cx: &mut Context<Self>) {
        log::debug!("BuildPanel::add_line called with: {}", line);
        let entry = BuildLogEntry {
            entry_type: BuildLogType::from_line(&line),
            line: line.clone().into(),
        };

        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
        log::debug!("BuildPanel now has {} log entries", self.logs.len());

        // Append to buffer
        let log_line = format!("{}\n", line);
        self.log_buffer.update(cx, |buffer, cx| {
            buffer.edit([(buffer.len()..buffer.len(), log_line)], None, cx);
        });

        // Auto-scroll is handled by the editor

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
        self.log_buffer.update(cx, |buffer, cx| {
            let len = buffer.len();
            buffer.edit([(0..len, "")], None, cx);
        });
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
        log::info!("BuildPanel::render called - log_count: {}", self.logs.len());

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
            .id("build-panel")
            .key_context("BuildPanel")
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

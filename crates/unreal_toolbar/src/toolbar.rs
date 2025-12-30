use crate::actions::{DebugStart, LiveCodingBuild, PieStart, PieStop};
use cherry_link::{BuildConfiguration, CherryLinkConnection, PlayState};
use gpui::{
    div, prelude::*, px, App, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window,
};
use ui::{
    prelude::*, Button, ButtonStyle, IconButton, IconName, PopoverMenu, Tooltip,
};
use workspace::Workspace;

/// Unreal Engine toolbar component
pub struct UnrealToolbar {
    workspace: WeakEntity<Workspace>,
    connection: Option<Entity<CherryLinkConnection>>,
    selected_config: BuildConfiguration,
    visible: bool,
    _subscriptions: Vec<Subscription>,
}

impl UnrealToolbar {
    pub fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            connection: None,
            selected_config: BuildConfiguration::default(),
            visible: true,
            _subscriptions: Vec::new(),
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

    fn render_play_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);
        let play_state = self.play_state(cx);

        let (icon, tooltip, action_name) = match play_state {
            PlayState::Playing | PlayState::Simulating => {
                (IconName::Stop, "Stop PIE", "Stop")
            }
            PlayState::Paused => {
                (IconName::PlayFilled, "Resume PIE", "Resume")
            }
            PlayState::Stopped => {
                (IconName::PlayFilled, "Start PIE", "Play")
            }
        };

        IconButton::new("pie-play", icon)
            .icon_size(IconSize::Small)
            .disabled(!is_connected)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(connection) = &this.connection {
                    connection.update(cx, |conn, cx| {
                        match play_state {
                            PlayState::Playing | PlayState::Simulating => conn.play_stop(cx),
                            _ => conn.play_start(cx),
                        }
                    });
                }
            }))
    }

    fn render_build_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);

        IconButton::new("build", IconName::ToolHammer)
            .icon_size(IconSize::Small)
            .disabled(!is_connected)
            .tooltip(Tooltip::text("Live Coding Build"))
            .on_click(cx.listener(|this, _, window, cx| {
                if let Some(connection) = &this.connection {
                    connection.update(cx, |conn, cx| {
                        conn.build_live_coding(cx);
                    });
                }
            }))
    }

    fn render_debug_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);

        IconButton::new("debug", IconName::Debug)
            .icon_size(IconSize::Small)
            .disabled(!is_connected)
            .tooltip(Tooltip::text("Start Debug Session"))
            .on_click(cx.listener(|this, _, window, cx| {
                // TODO: Implement debug action
            }))
    }

    fn render_config_dropdown(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_config;

        Button::new("config-dropdown", selected.to_string())
            .style(ButtonStyle::Subtle)
            .tooltip(Tooltip::text("Build Configuration"))
            .on_click(cx.listener(|this, _, window, cx| {
                // Cycle through configurations for now
                let configs = BuildConfiguration::all();
                let current_idx = configs.iter().position(|c| *c == this.selected_config).unwrap_or(0);
                let next_idx = (current_idx + 1) % configs.len();
                this.selected_config = configs[next_idx];
                cx.notify();
            }))
    }

    fn render_connection_status(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected = self.is_connected(cx);
        let (icon, color, tooltip) = if is_connected {
            (IconName::Check, ui::Color::Success, "Connected to Unreal Engine")
        } else {
            (IconName::XCircle, ui::Color::Error, "Disconnected from Unreal Engine")
        };

        IconButton::new("connection-status", icon)
            .icon_size(IconSize::Small)
            .icon_color(color)
            .tooltip(Tooltip::text(tooltip))
            .on_click(cx.listener(|this, _, window, cx| {
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
}

impl Render for UnrealToolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        div()
            .id("unreal-toolbar")
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .bg(cx.theme().colors().toolbar_background)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(self.render_play_button(window, cx))
                    .child(self.render_build_button(window, cx))
                    .child(self.render_debug_button(window, cx))
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .ml_2()
                    .child(self.render_config_dropdown(window, cx))
            )
            .child(
                div()
                    .flex_grow()
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(self.render_connection_status(window, cx))
            )
            .into_any_element()
    }
}

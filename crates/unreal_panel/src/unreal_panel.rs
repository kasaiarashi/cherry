mod build_panel;
mod panel;

pub use build_panel::BuildPanel;
pub use panel::UnrealPanel;

use gpui::{actions, App};

actions!(
    unreal_panel,
    [
        /// Toggle the Unreal Engine output panel
        ToggleFocus,
        /// Toggle the Build output panel
        ToggleBuildPanel,
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut workspace::Workspace, _window, _: &mut gpui::Context<workspace::Workspace>| {
            workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
                workspace.toggle_panel_focus::<UnrealPanel>(window, cx);
            });
            workspace.register_action(|workspace, _: &ToggleBuildPanel, window, cx| {
                workspace.toggle_panel_focus::<BuildPanel>(window, cx);
            });
        },
    )
    .detach();
}

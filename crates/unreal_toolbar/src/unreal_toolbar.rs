mod actions;
mod toolbar;

pub use actions::*;
pub use toolbar::UnrealToolbar;

use gpui::{App, AppContext};
use workspace::Workspace;

pub fn init(cx: &mut App) {
    actions::init(cx);

    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        let toolbar = cx.new(|cx| UnrealToolbar::new(workspace.weak_handle(), cx));
        workspace.set_toolbar_item(toolbar.into(), window, cx);
    });
}

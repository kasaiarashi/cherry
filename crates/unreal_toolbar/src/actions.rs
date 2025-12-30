use gpui::{actions, App};

actions!(
    unreal,
    [
        /// Start Play-In-Editor session
        PieStart,
        /// Stop PIE session
        PieStop,
        /// Pause PIE session
        PiePause,
        /// Resume PIE session
        PieResume,
        /// Trigger Live Coding build
        LiveCodingBuild,
        /// Trigger full project build
        FullBuild,
        /// Start debug session
        DebugStart,
        /// Toggle Unreal toolbar visibility
        ToggleUnrealToolbar,
    ]
);

pub fn init(cx: &mut App) {
    // Register global action handlers if needed
}

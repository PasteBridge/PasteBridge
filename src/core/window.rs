//! Window management - platform agnostic
//! 
//! Handles window position calculation, show/hide logic

use std::sync::Arc;
use crate::core::state::AppState;

/// Window configuration
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 280,
            height: 396,
        }
    }
}

/// Calculate window position using golden ratio centering
pub fn calculate_center_position(_config: &WindowConfig) -> (i32, i32) {
    // Screen dimensions would come from platform-specific code
    // For now, return placeholder - will be filled by platform implementation
    (0, 0)
}

/// Calculate position to hide window off-screen
pub fn calculate_hidden_position() -> (i32, i32) {
    (-10000, -10000)
}

/// Toggle window visibility
pub fn toggle_window_visibility<F>(state: Arc<AppState>, on_show: F, on_hide: F)
where
    F: Fn() + Clone,
{
    if state.is_window_visible() {
        on_hide();
        state.set_window_visible(false);
    } else {
        on_show();
        state.set_window_visible(true);
    }
}

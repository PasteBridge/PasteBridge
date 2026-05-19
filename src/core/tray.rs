//! Tray icon management

use std::sync::atomic::{AtomicBool, Ordering};

/// Global visibility state for tray
pub static IS_VISIBLE: AtomicBool = AtomicBool::new(true);

/// Tray menu event
#[derive(Debug, Clone)]
pub enum TrayEvent {
    Show,
    Hide,
    Quit,
}

/// Check if window should be visible
pub fn is_visible() -> bool {
    IS_VISIBLE.load(Ordering::SeqCst)
}

/// Set visibility state
pub fn set_visible(visible: bool) {
    IS_VISIBLE.store(visible, Ordering::SeqCst);
}

//! Windows window implementation

use std::sync::atomic::AtomicIsize;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

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

/// Global window handle storage
pub static APP_HWND: AtomicIsize = AtomicIsize::new(0);

/// Calculate screen dimensions
pub fn get_screen_size() -> (i32, i32) {
    unsafe {
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    }
}

/// Calculate golden ratio centered position
pub fn calculate_golden_position(config: &WindowConfig) -> (i32, i32) {
    let (screen_w, screen_h) = get_screen_size();
    let x = (screen_w - config.width) / 2;
    // Window 0.382 aligned with screen 0.618
    let y = (((screen_h as f64) * 0.618) - (config.height as f64) * 0.233) as i32;
    (x, y)
}

/// Windows window operations
pub struct WindowsWindow {
    pub config: WindowConfig,
}

impl WindowsWindow {
    pub fn new() -> Self {
        Self {
            config: WindowConfig::default(),
        }
    }

    pub fn show_at_center(&self) -> (i32, i32) {
        calculate_golden_position(&self.config)
    }

    pub fn hide_position() -> (i32, i32) {
        (-10000, -10000)
    }
}

impl Default for WindowsWindow {
    fn default() -> Self {
        Self::new()
    }
}

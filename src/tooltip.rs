use std::sync::atomic::{AtomicIsize, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Tooltip window handle
pub static TOOLTIP_HWND: AtomicIsize = AtomicIsize::new(0);

/// Z-order monitor running flag
pub static ZORDER_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

/// Current tooltip content
static TOOLTIP_TEXT: Mutex<String> = Mutex::new(String::new());
static TOOLTIP_TIMESTAMP: Mutex<String> = Mutex::new(String::new());

/// Set tooltip window handle
pub fn set_tooltip_hwnd(hwnd: isize) {
    TOOLTIP_HWND.store(hwnd, Ordering::SeqCst);
}

/// Ensure tooltip window is always on top of the main window
pub fn ensure_tooltip_always_on_top() {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::*;

            let tooltip_hwnd = TOOLTIP_HWND.load(Ordering::SeqCst);
            let app_hwnd = crate::window_effects::APP_HWND.load(Ordering::SeqCst);

            if tooltip_hwnd != 0 && app_hwnd != 0 {
                let tooltip = HWND(tooltip_hwnd as *mut std::ffi::c_void);
                let app = HWND(app_hwnd as *mut std::ffi::c_void);

                // Use HWND_TOPMOST to ensure tooltip is always on top
                // This prevents the main window from covering tooltip when clicked
                SetWindowPos(
                    tooltip,
                    HWND_TOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                ).ok();

                // Then set it relative to main window to keep it above main window specifically
                SetWindowPos(
                    tooltip,
                    app,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                ).ok();
            }
        }
    }
}

/// Start z-order monitor to keep tooltip on top of main window
pub fn start_tooltip_zorder_monitor() {
    if ZORDER_MONITOR_RUNNING.load(Ordering::SeqCst) {
        return;
    }

    ZORDER_MONITOR_RUNNING.store(true, Ordering::SeqCst);

    thread::spawn(move || {
        loop {
            if !ZORDER_MONITOR_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // Ensure tooltip is on top of main window
            ensure_tooltip_always_on_top();

            thread::sleep(Duration::from_millis(100));
        }
    });

    eprintln!("[tooltip] Z-order monitor started");
}

/// Stop z-order monitor
pub fn stop_tooltip_zorder_monitor() {
    ZORDER_MONITOR_RUNNING.store(false, Ordering::SeqCst);
}

/// Start tooltip system
pub fn start_tooltip() {
    start_tooltip_zorder_monitor();
    eprintln!("[tooltip] Tooltip system started");
}

/// Create tooltip window (this is handled by Slint's PopupTooltipWindow)
pub fn create_tooltip_window() {
    eprintln!("[tooltip] Tooltip window creation handled by Slint");
}

/// Set tooltip content
pub fn set_tooltip_content(text: &str, timestamp: &str) {
    let mut tooltip_text = TOOLTIP_TEXT.lock().unwrap();
    let mut tooltip_timestamp = TOOLTIP_TIMESTAMP.lock().unwrap();
    *tooltip_text = text.to_string();
    *tooltip_timestamp = timestamp.to_string();
}

/// Show simple tooltip at position (for "Copied" notification)
pub fn show_tooltip_at(_x: i32, _y: i32, _text: &str) {
    #[cfg(target_os = "windows")]
    {
        // This is currently handled by Slint's popup
        eprintln!("[tooltip] Show simple tooltip at ({}, {}): {}", _x, _y, _text);
    }
}

/// Hide tooltip
pub fn hide_tooltip() {
    // Handled by Slint's PopupTooltipWindow
    eprintln!("[tooltip] Hide tooltip");
}

/// Destroy tooltip window
pub fn destroy_tooltip_window() {
    stop_tooltip_zorder_monitor();
    eprintln!("[tooltip] Tooltip window destroyed");
}

/// Destroy hover tooltip window
pub fn destroy_hover_tooltip_window() {
    // Same as destroy_tooltip_window
    destroy_tooltip_window();
}

/// Get cursor position (duplicate of window_effects for compatibility)
pub fn get_cursor_pos() -> (i32, i32) {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::*;
            
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point).is_ok() {
                return (point.x, point.y);
            }
        }
    }
    (0, 0)
}

/// Hover callback storage
type HoverCallback = Box<dyn Fn(i32, i32) -> Option<String> + Send + Sync + 'static>;
static HOVER_CALLBACK: Mutex<Option<Arc<HoverCallback>>> = Mutex::new(None);

/// Start hover detection
pub fn start_hover_detection() {
    eprintln!("[tooltip] Hover detection started (handled by Slint)");
}

/// Update hover state
pub fn update_hover_state(_y_offset: i32, _item_height: i32) {
    // Handled by Slint UI
}

/// Set hover callback
pub fn set_hover_callback<F>(callback: F)
where
    F: Fn(i32, i32) -> Option<String> + Send + Sync + 'static
{
    let mut cb = HOVER_CALLBACK.lock().unwrap();
    *cb = Some(Arc::new(Box::new(callback)));
    eprintln!("[tooltip] Hover callback set");
}

/// Show hover tooltip at position (with content and timestamp)
pub fn show_hover_tooltip_at(_x: i32, _y: i32, _content: String, _timestamp: String) {
    // This is handled by Slint's PopupTooltipWindow
    set_tooltip_content(&_content, &_timestamp);
}

/// Hide hover tooltip
pub fn hide_hover_tooltip() {
    hide_tooltip();
}

/// Bring tooltip to front explicitly
pub fn bring_tooltip_to_front() {
    ensure_tooltip_always_on_top();
}


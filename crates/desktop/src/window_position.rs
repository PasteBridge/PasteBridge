use crate::focus;
use crate::AppWindow;

#[cfg(target_os = "windows")]
pub mod window_util {
    use std::os::raw::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn ReleaseCapture() -> i32;
        fn SendMessageW(hwnd: *mut c_void, msg: u32, wparam: u32, lparam: isize) -> isize;
    }

    pub fn start_drag(hwnd: *mut c_void) {
        unsafe {
            ReleaseCapture();
            SendMessageW(hwnd, 0x112, 0xF012, 0);
            SendMessageW(hwnd, 0x0202, 0, 0);
        }
    }
}

pub fn calc_window_position(app: &AppWindow, win_w: i32, win_h: i32) -> slint::PhysicalPosition {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let y_offset: i32 = 14;

    let caret = focus::get_cached_focus_pos()
        .or_else(|| focus::get_focused_caret_screen_pos());

    if let Some(caret) = caret {
        let mut x = caret.left;
        let mut y = caret.bottom + y_offset;

        if x + win_w > screen_w {
            x = caret.right - win_w;
        }
        if x < 0 {
            x = 0;
        }

        if y + win_h > screen_h {
            y = caret.top - win_h - y_offset;
        }
        if y < 0 {
            y = 0;
        }

        return slint::PhysicalPosition::new(x, y);
    }

    let mode = app.get_window_position_mode();

    match mode {
        0 => {
            let x = (screen_w - win_w) / 2;
            let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;
            slint::PhysicalPosition::new(x, y)
        }
        _ => {
            let (cursor_x, cursor_y) = crate::window_effects::get_cursor_pos();
            let x = cursor_x - win_w / 2;
            let y = cursor_y - win_h / 2;
            slint::PhysicalPosition::new(x, y)
        }
    }
}
use crate::AppWindow;
use slint::ComponentHandle;

pub fn trigger_content_update_fade(weak: slint::Weak<AppWindow>) {
    if let Some(w) = weak.upgrade() {
        w.set_main_content_fade_out(true);
        let w_clone = w.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
            if let Some(w) = w_clone.upgrade() {
                w.set_main_content_fade_out(false);
            }
        });
    }
}

#[cfg(target_os = "windows")]
pub fn fade_window_out() {
    use std::sync::atomic::Ordering;
    let hwnd_isize = crate::window_effects::APP_HWND.load(Ordering::SeqCst);
    if hwnd_isize != 0 {
        let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
        crate::window_effects::fade_out(hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn fade_window_out() {}

#[cfg(target_os = "windows")]
pub fn fade_window_in() {
    use std::sync::atomic::Ordering;
    let hwnd_isize = crate::window_effects::APP_HWND.load(Ordering::SeqCst);
    if hwnd_isize != 0 {
        let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
        crate::window_effects::fade_in(hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn fade_window_in() {}

#[cfg(target_os = "windows")]
pub fn startup_window_fade_in() {
    std::thread::spawn(move || {
        crate::window_effects::wait_for_window_effects_ready();
        let _ = slint::invoke_from_event_loop(move || {
            let hwnd_isize = crate::window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                crate::window_effects::fade_in(hwnd);
            }
        });
    });
}

#[cfg(not(target_os = "windows"))]
pub fn startup_window_fade_in() {}
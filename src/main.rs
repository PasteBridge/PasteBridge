slint::include_modules!();

mod clipboard;
mod window_effects;
mod tray;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

#[cfg(target_os = "windows")]
mod window_util {
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

fn main() {
    // 尝试使用默认后端（通常支持硬件加速），如果不行再回退到软件渲染
    std::env::set_var("SLINT_BACKEND", "winit-software");
    std::env::set_var("SLINT_STYLE", "fluent");

    eprintln!("Starting PasteBridge...");

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    // 启动剪贴板监控
    clipboard::start_clipboard_monitor(app_weak.clone(), 20);

    // 应用窗口效果（毛玻璃、阴影）
    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();

    // 设置全局热键 (Ctrl + Shift + V)
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
    manager.register(hotkey).unwrap();
    let hotkey_id = hotkey.id();

    // 设置托盘图标
    let handles = tray::setup_tray();
    // 保持 tray_icon 存活，否则图标会消失
    let _tray_icon = handles.tray_icon;
    tray::start_tray_event_loop(handles.show_id, handles.quit_id, hotkey_id);

    // 设置窗口操作回调
    let weak4 = app_weak.clone();
    app.on_start_drag(move || {
        if let Some(_w) = weak4.upgrade() {
            #[cfg(target_os = "windows")]
            {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                    let hwnd = GetForegroundWindow();
                    window_util::start_drag(hwnd.0);
                }
            }
        }
    });

    let _weak2 = app_weak.clone();
    app.on_hide_window(move || {
        let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
        if hwnd_isize != 0 {
            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE);
            }
            tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });

    app.on_minimize_window(move || {
        let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
        if hwnd_isize != 0 {
            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE);
            }
            tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });

    eprintln!("About to run app...");
    app.run().ok();
    eprintln!("App run() finished");
}

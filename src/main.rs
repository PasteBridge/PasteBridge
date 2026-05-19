slint::include_modules!();

slint::slint! {
    export component DummyWindow inherits Window {
        width: 1px;
        height: 1px;
        no-frame: true;
        background: transparent;
    }
}

mod clipboard;
mod window_effects;
mod tray;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN;
use windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN;

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

    // 设置全局热键 (Ctrl + Alt + V)
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
    let hotkey_id = match manager.register(hotkey) {
        Ok(_) => hotkey.id(),
        Err(e) => {
            eprintln!("热键 Ctrl+Alt+V 已被占用，尝试使用 Ctrl+Alt+B... ({e})");
            // 尝试备选热键 Ctrl+Alt+B
            let backup_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyB);
            match manager.register(backup_hotkey) {
                Ok(_) => backup_hotkey.id(),
                Err(e2) => {
                    eprintln!("备选热键也失败: {e2}");
                    tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
                    std::process::exit(1);
                }
            }
        }
    };

    // 设置托盘图标
    let handles = tray::setup_tray();
    // 保持 tray_icon 存活，否则图标会消失
    let _tray_icon = handles.tray_icon;
    let weak_for_tray = app_weak.clone();
    tray::start_tray_event_loop(handles.show_id, handles.quit_id, hotkey_id, move || {
        let _ = slint::invoke_from_event_loop({
            let weak = weak_for_tray.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    use slint::ComponentHandle;
                    let is_visible = tray::IS_VISIBLE.load(std::sync::atomic::Ordering::SeqCst);
                    if is_visible {
                        // 淡出 → 再移出屏幕
                        let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            window_effects::fade_out(hwnd);
                        }
                        let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
                        tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
                    } else {
                        // 窗口中心点位于屏幕高度的 0.618 处，水平居中
                        let win_w = 280i32;
                        let win_h = 396i32;
                        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
                        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
                        let x = (screen_w - win_w) / 2;
                        // 窗口高度的 0.382 处对齐屏幕高度的 0.618
                        let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;

                        let _ = app.window().set_position(slint::PhysicalPosition::new(x, y));
                        tray::IS_VISIBLE.store(true, std::sync::atomic::Ordering::SeqCst);
                        
                        let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            // 先 SetForegroundWindow，再异步淡入，避免阻塞事件循环
                            unsafe {
                                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                            }
                            window_effects::fade_in(hwnd);
                        }
                    }
                }
            }
        });
    });

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

    let weak2 = app_weak.clone();
    app.on_hide_window(move || {
        if let Some(app) = weak2.upgrade() {
            use slint::ComponentHandle;
            let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });

    // 复制条目回调
    app.on_copy_item(move |text: slint::SharedString| {
        let text_owned: String = text.into();
        clipboard::set_clipboard_text(text_owned.clone());
        eprintln!("Copied to clipboard: {}", text_owned.chars().take(20).collect::<String>());
    });

    let weak3 = app_weak.clone();
    app.on_minimize_window(move || {
        if let Some(app) = weak3.upgrade() {
            use slint::ComponentHandle;
            let hwnd_isize = window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });

    eprintln!("About to run app...");
    app.run().ok();
    eprintln!("App run() finished");
}

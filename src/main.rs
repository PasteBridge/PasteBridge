slint::include_modules!();

slint::slint! {
    export component DummyWindow inherits Window {
        width: 1px;
        height: 1px;
        no-frame: true;
        background: transparent;
    }
}

// Re-use existing modules
pub mod clipboard;
pub mod window_effects;
pub mod tray;
pub mod tooltip;

use std::sync::atomic::Ordering;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

// Re-export modules
mod api;
mod core;

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
    // Set backend and style
    std::env::set_var("SLINT_BACKEND", "winit-software");
    std::env::set_var("SLINT_STYLE", "fluent");

    eprintln!("Starting PasteBridge...");

    // Get app data directory
    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Create app state with database
    let state = core::state::AppState::new(&app_data_dir, 100);

    // Create and setup app window
    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    // Load clipboard history from database on startup
    let state_for_init = state.clone();
    let app_for_init = app.as_weak();
    slint::invoke_from_event_loop(move || {
        if let Some(w) = app_for_init.upgrade() {
            let history = state_for_init.get_history();
            let items: Vec<String> = history.iter()
                .filter_map(|item| item.content_text.clone())
                .collect();
            let items: Vec<slint::SharedString> = items.into_iter().map(|s| s.into()).collect();
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            w.set_clipboard_history(model.into());
        }
    }).ok();

    // Start clipboard monitoring - update UI when clipboard changes
    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    core::clipboard::start_clipboard_monitor(state_for_clipboard, move |_text| {
        let weak = app_weak_clone.clone();
        let state = state_for_ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = weak.upgrade() {
                let history = state.get_history();
                let items: Vec<String> = history.iter()
                    .filter_map(|item| item.content_text.clone())
                    .collect();
                let items: Vec<slint::SharedString> = items.into_iter().map(|s| s.into()).collect();
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                w.set_clipboard_history(model.into());
            }
        });
    });

    // Apply window effects (blur, shadow)
    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();
    
    // Create tooltip window
    #[cfg(target_os = "windows")]
    tooltip::create_tooltip_window();

    // Setup global hotkey (Ctrl+Alt+V, fallback Ctrl+Alt+B)
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
    let hotkey_id = match manager.register(hotkey) {
        Ok(_) => hotkey.id(),
        Err(e) => {
            eprintln!("Hotkey Ctrl+Alt+V occupied, trying Ctrl+Alt+B... ({})", e);
            let backup_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyB);
            match manager.register(backup_hotkey) {
                Ok(_) => backup_hotkey.id(),
                Err(e2) => {
                    eprintln!("Backup hotkey also failed: {}", e2);
                    tray::IS_VISIBLE.store(false, Ordering::SeqCst);
                    std::process::exit(1);
                }
            }
        }
    };

    // Setup tray icon
    let handles = tray::setup_tray();
    let _tray_icon = handles.tray_icon;
    let weak_for_tray = app_weak.clone();
    tray::start_tray_event_loop(handles.show_id, handles.quit_id, hotkey_id, move || {
        let _ = slint::invoke_from_event_loop({
            let weak = weak_for_tray.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    use slint::ComponentHandle;
                    let is_visible = tray::IS_VISIBLE.load(Ordering::SeqCst);
                    if is_visible {
                        // Hide window
                        let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            window_effects::fade_out(hwnd);
                        }
                        let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
                        tray::IS_VISIBLE.store(false, Ordering::SeqCst);
                    } else {
                        // Show window at golden ratio position
                        let win_w = 280i32;
                        let win_h = 396i32;
                        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
                        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
                        let x = (screen_w - win_w) / 2;
                        let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;

                        let _ = app.window().set_position(slint::PhysicalPosition::new(x, y));
                        tray::IS_VISIBLE.store(true, Ordering::SeqCst);

                        let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
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

    // Setup window callbacks
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
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);
        }
    });

    // Copy item callback
    app.on_copy_item(move |text: slint::SharedString| {
        let text_owned: String = text.into();
        core::clipboard::set_clipboard_text(text_owned.clone());
        eprintln!("Copied to clipboard: {}", text_owned.chars().take(20).collect::<String>());
        
        // Show tooltip at cursor position
        #[cfg(target_os = "windows")]
        {
            let pos = tooltip::get_cursor_pos();
            tooltip::show_tooltip_at(pos.0, pos.1, "Copied");
        }
    });

    // Clear history callback
    let state_for_clear = state.clone();
    let app_for_clear = app.as_weak();
    app.on_clear_history(move || {
        state_for_clear.clear_history();
        let app_clone = app_for_clear.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = app_clone.upgrade() {
                let model = std::rc::Rc::new(slint::VecModel::<slint::SharedString>::from(vec![]));
                w.set_clipboard_history(model.into());
            }
        });
        eprintln!("Clipboard history cleared");
    });

    let weak3 = app_weak.clone();
    app.on_minimize_window(move || {
        if let Some(app) = weak3.upgrade() {
            use slint::ComponentHandle;
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);
        }
    });

    // Start API server (runs on 18792)
    let api_state = state.clone();
    std::thread::spawn(move || {
        let mut server = api::ApiServer::new(18792);
        if let Err(e) = server.start(api_state) {
            eprintln!("[api] Server error: {}", e);
        }
    });

    eprintln!("About to run app...");
    app.run().ok();
    eprintln!("App run() finished");
}

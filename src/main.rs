slint::include_modules!();

slint::slint! {
    export component DummyWindow inherits Window {
        width: 1px;
        height: 1px;
        no-frame: true;
        background: transparent;
    }
}

pub mod clipboard;
pub mod window_effects;
pub mod tray;
pub mod tooltip;
pub mod ui;
pub mod platform;

use std::sync::atomic::Ordering;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

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
    std::env::set_var("SLINT_BACKEND", "winit-skia");
    std::env::set_var("SLINT_STYLE", "fluent");
    std::env::set_var("ICU4X_DATA_DIR", "");

    const WINDOW_WIDTH: f32 = 280.0;
    const WINDOW_HEIGHT: f32 = 396.0;
    const HIDDEN_WINDOW_SIZE: f32 = 1.0;

    eprintln!("Starting PasteBridge...");

    let memory_monitor = paste_bridge_core::memory::MemoryMonitor::new();
    let initial_memory = memory_monitor.update();
    eprintln!("[memory] Initial memory: {}", paste_bridge_core::memory::MemoryMonitor::format_memory(initial_memory));

    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let state = paste_bridge_core::state::AppState::new(&app_data_dir, 10);

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    let win_w = WINDOW_WIDTH as i32;
    let win_h = WINDOW_HEIGHT as i32;
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let x = (screen_w - win_w) / 2;
    let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;
    let _ = app.window().set_position(slint::PhysicalPosition::new(x, y));
    tray::IS_VISIBLE.store(true, Ordering::SeqCst);

    use std::sync::Arc;
    #[derive(Clone)]
    struct ClipboardEntry {
        text: slint::SharedString,
        id: i64,
    }
    let clipboard_entries: Arc<std::sync::Mutex<Vec<ClipboardEntry>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let clipboard_entries_clone = clipboard_entries.clone();

    let state_for_init = state.clone();
    let app_for_init = app.as_weak();
    let entries_for_init = clipboard_entries_clone.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(w) = app_for_init.upgrade() {
            let history = state_for_init.get_history();
            let entries: Vec<ClipboardEntry> = history.iter()
                .filter_map(|item| {
                    item.content_text.clone().map(|text| ClipboardEntry {
                        text: text.into(),
                        id: item.id,
                    })
                })
                .collect();

            {
                let mut entries_lock = entries_for_init.lock().unwrap();
                *entries_lock = entries.clone();
            }

            let items: Vec<slint::SharedString> = entries.iter().map(|e| e.text.clone()).collect();
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            w.set_clipboard_history(model.into());
        }
    }).ok();

    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    let entries_for_update = clipboard_entries_clone.clone();
    let memory_monitor_clone = std::sync::Arc::new(memory_monitor);
    let mem_for_update = memory_monitor_clone.clone();
    clipboard::start_clipboard_monitor(state_for_clipboard, move || {
        let weak = app_weak_clone.clone();
        let state = state_for_ui.clone();
        let entries_for_update = entries_for_update.clone();
        let mem = mem_for_update.clone();

        let _ = slint::invoke_from_event_loop(move || {
            static UPDATE_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let count = UPDATE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let mem_before = if count % 10 == 0 { Some(mem.update()) } else { None };

            if let Some(w) = weak.upgrade() {
                let history = state.get_history();
                let entries: Vec<ClipboardEntry> = history.iter()
                    .filter_map(|item| {
                        item.content_text.clone().map(|text| ClipboardEntry {
                            text: text.into(),
                            id: item.id,
                        })
                    })
                    .collect();

                let items: Vec<slint::SharedString> = entries.iter()
                    .map(|e| e.text.clone())
                    .collect();

                {
                    let mut entries_lock = entries_for_update.lock().unwrap();
                    *entries_lock = entries;
                }

                let model = std::rc::Rc::new(slint::VecModel::from(items));
                w.set_clipboard_history(model.into());

                if let Some(before) = mem_before {
                    let mem_after = mem.update();
                    let mem_delta = if mem_after > before { mem_after - before } else { 0 };
                    eprintln!("[memory] Update {}: {} (+{})",
                        count,
                        paste_bridge_core::memory::MemoryMonitor::format_memory(mem_after),
                        paste_bridge_core::memory::MemoryMonitor::format_memory(mem_delta));
                }
            }
        });
    });

    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();

    #[cfg(target_os = "windows")]
    {
        let _app_for_fade = app_weak.clone();
        std::thread::spawn(move || {
            window_effects::wait_for_window_effects_ready();
            let _ = slint::invoke_from_event_loop(move || {
                let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                if hwnd_isize != 0 {
                    let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                    window_effects::fade_in(hwnd);
                }
            });
        });
    }

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
                    eprintln!("A previous instance might be running or hotkeys are used elsewhere. We will continue without a hotkey.");
                    0
                }
            }
        }
    };

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
                        // Hide: shrink + fade out + move off-screen
                        let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            window_effects::fade_out(hwnd);
                        }
                        app.window().set_size(slint::LogicalSize::new(HIDDEN_WINDOW_SIZE, HIDDEN_WINDOW_SIZE));
                        let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
                        tray::IS_VISIBLE.store(false, Ordering::SeqCst);
                    } else {
                        // Show: restore size + position + bring to front + fade in
                        app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
                        let win_w = WINDOW_WIDTH as i32;
                        let win_h = WINDOW_HEIGHT as i32;
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
    let mem_for_hide = memory_monitor_clone.clone();
    app.on_hide_window(move || {
        if let Some(app) = weak2.upgrade() {
            use slint::ComponentHandle;
            app.window().set_size(slint::LogicalSize::new(HIDDEN_WINDOW_SIZE, HIDDEN_WINDOW_SIZE));
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);

            // Trim working set to release idle memory
            if let Some(freed) = mem_for_hide.trim_working_set() {
                eprintln!("[memory] Working set trimmed: {} freed",
                    paste_bridge_core::memory::MemoryMonitor::format_memory(freed));
            }
        }
    });

    let entries_for_copy = clipboard_entries_clone.clone();
    let state_for_copy = state.clone();
    app.on_copy_item(move |index: i32| {
        let idx = index as usize;
        let entries = entries_for_copy.lock().unwrap();
        if idx >= entries.len() {
            eprintln!("copy-item: index out of range: {}", idx);
            return;
        }
        let id = entries[idx].id;
        if let Some(item) = state_for_copy.get_item(id) {
            if let Some(text) = item.content_text {
                paste_bridge_core::clipboard::set_clipboard_text(text.clone());
                eprintln!("Copied to clipboard (id={}): {}", id, text.chars().take(20).collect::<String>());
                #[cfg(target_os = "windows")]
                {
                    let pos = tooltip::get_cursor_pos();
                    tooltip::show_tooltip_at(pos.0, pos.1, "Copied");
                }
            } else {
                eprintln!("copy-item: item id {} has no text", id);
            }
        } else {
            eprintln!("copy-item: no item found for id {}", id);
        }
    });

    let entries_for_hover = clipboard_entries_clone.clone();
    let state_for_hover = state.clone();
    app.on_show_hover_tooltip_index(move |index: i32| {
        let idx = index as usize;
        let entries = entries_for_hover.lock().unwrap();
        if idx >= entries.len() {
            return;
        }
        let id = entries[idx].id;
        if let Some(item) = state_for_hover.get_item(id) {
            if let Some(text) = item.content_text {
                #[cfg(target_os = "windows")]
                {
                    tooltip::show_hover_tooltip(&text);
                }
            }
        }
    });

    app.on_hide_hover_tooltip(move || {
        #[cfg(target_os = "windows")]
        {
            tooltip::hide_hover_tooltip();
        }
    });

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
    let mem_for_minimize = memory_monitor_clone.clone();
    app.on_minimize_window(move || {
        if let Some(app) = weak3.upgrade() {
            use slint::ComponentHandle;
            app.window().set_size(slint::LogicalSize::new(HIDDEN_WINDOW_SIZE, HIDDEN_WINDOW_SIZE));
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);

            // Trim working set to release idle memory
            if let Some(freed) = mem_for_minimize.trim_working_set() {
                eprintln!("[memory] Working set trimmed: {} freed",
                    paste_bridge_core::memory::MemoryMonitor::format_memory(freed));
            }
        }
    });

    app.on_quit_app(|| {
        eprintln!("Quit requested, exiting application...");
        std::process::exit(0);
    });

    let app_for_settings = app.as_weak();
    app.on_toggle_settings(move || {
        if let Some(app) = app_for_settings.upgrade() {
            use slint::ComponentHandle;
            let current = app.get_settings_visible();
            app.set_settings_visible(!current);
            if current {
                app.window().request_redraw();
            }
        }
    });

    let api_state = state.clone();
    std::thread::spawn(move || {
        let mut server = paste_bridge_core::api::ApiServer::new(18792);
        if let Err(e) = server.start(api_state) {
            eprintln!("[api] Server error: {}", e);
        }
    });

    eprintln!("About to run app...");
    app.run().ok();
    eprintln!("App run() finished");
}
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
    // Set backend to Skia renderer for full feature support including Chinese text
    std::env::set_var("SLINT_BACKEND", "winit-skia");
    std::env::set_var("SLINT_STYLE", "fluent");
    
    // Disable ICU4X verbose logging to reduce memory overhead
    std::env::set_var("ICU4X_DATA_DIR", "");

    const WINDOW_WIDTH: f32 = 280.0;
    const WINDOW_HEIGHT: f32 = 396.0;
    const HIDDEN_WINDOW_SIZE: f32 = 1.0;

    eprintln!("Starting PasteBridge...");

    // Initialize memory monitor
    let memory_monitor = core::memory::MemoryMonitor::new();
    let initial_memory = memory_monitor.update();
    eprintln!("[memory] Initial memory: {}", core::memory::MemoryMonitor::format_memory(initial_memory));

    // Get app data directory
    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Create app state with database (limit to 10 items for reduced memory usage)
    let state = core::state::AppState::new(&app_data_dir, 10);

    // Create and setup app window
    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();
    
    // Set initial window size and position
    app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    let win_w = WINDOW_WIDTH as i32;
    let win_h = WINDOW_HEIGHT as i32;
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let x = (screen_w - win_w) / 2;
    let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;
    let _ = app.window().set_position(slint::PhysicalPosition::new(x, y));
    tray::IS_VISIBLE.store(true, Ordering::SeqCst);
    
    // Single source of truth: store both text and IDs together
    use std::sync::Arc;
    #[derive(Clone)]
    struct ClipboardEntry {
        text: slint::SharedString,
        id: i64,
    }
    let clipboard_entries: Arc<std::sync::Mutex<Vec<ClipboardEntry>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let clipboard_entries_clone = clipboard_entries.clone();

    // Load clipboard history from database on startup
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
            
            // Update shared entries
            {
                let mut entries_lock = entries_for_init.lock().unwrap();
                *entries_lock = entries.clone();
            }
            
            let items: Vec<slint::SharedString> = entries.iter().map(|e| e.text.clone()).collect();
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            w.set_clipboard_history(model.into());
        }
    }).ok();

    // Start clipboard monitoring - update UI when clipboard changes
    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    let entries_for_update = clipboard_entries_clone.clone();
    let memory_monitor_clone = std::sync::Arc::new(memory_monitor);
    let mem_for_update = memory_monitor_clone.clone();
    core::clipboard::start_clipboard_monitor(state_for_clipboard, move || {
        let weak = app_weak_clone.clone();
        let state = state_for_ui.clone();
        let entries_for_update = entries_for_update.clone();
        let mem = mem_for_update.clone();
        
        let _ = slint::invoke_from_event_loop(move || {
            // Only track memory on every 10th update to reduce logging overhead
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
                
                // Extract items for UI model
                let items: Vec<slint::SharedString> = entries.iter()
                    .map(|e| e.text.clone())
                    .collect();
                
                // Update shared entries
                {
                    let mut entries_lock = entries_for_update.lock().unwrap();
                    *entries_lock = entries;
                }
                
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                w.set_clipboard_history(model.into());
                
                // Only log memory every 10 updates
                if let Some(before) = mem_before {
                    let mem_after = mem.update();
                    let mem_delta = if mem_after > before { mem_after - before } else { 0 };
                    eprintln!("[memory] Update {}: {} (+{})", 
                        count,
                        core::memory::MemoryMonitor::format_memory(mem_after),
                        core::memory::MemoryMonitor::format_memory(mem_delta));
                }
            }
        });
    });

    // Apply window effects (blur, shadow)
    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();
    
    // Wait for window effects to be ready and fade in
    #[cfg(target_os = "windows")]
    {
        let app_for_fade = app_weak.clone();
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
    
    // Setup global hotkey (Ctrl+Alt+V, fallback Ctrl+Alt+B)
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
    
    // We try catching the register errors. Sometimes it happens if another instance is already running.
    // So let's fall back to another key or simply not exit if both fail.
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
            app.window().set_size(slint::LogicalSize::new(HIDDEN_WINDOW_SIZE, HIDDEN_WINDOW_SIZE));
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);
        }
    });

    // Copy item callback (index-based) - load full item on demand
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
                core::clipboard::set_clipboard_text(text.clone());
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

    // Hover tooltip callbacks - index-based: load full text on demand for tooltip
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
            app.window().set_size(slint::LogicalSize::new(HIDDEN_WINDOW_SIZE, HIDDEN_WINDOW_SIZE));
            let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
            if hwnd_isize != 0 {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                window_effects::fade_out(hwnd);
            }
            let _ = app.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            tray::IS_VISIBLE.store(false, Ordering::SeqCst);
        }
    });

    app.on_quit_app(|| {
        eprintln!("Quit requested, exiting application...");
        std::process::exit(0);
    });

    // Toggle settings callback
    let app_for_settings = app.as_weak();
    app.on_toggle_settings(move || {
        if let Some(app) = app_for_settings.upgrade() {
            use slint::ComponentHandle;
            let current = app.get_settings_visible();
            app.set_settings_visible(!current);
            // When hiding settings, ask the renderer to discard stale cached data.
            if current {
                app.window().request_redraw();
            }
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

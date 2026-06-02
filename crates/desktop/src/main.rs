slint::include_modules!();

slint::slint! {
    export component DummyWindow inherits Window {
        width: 1px;
        height: 1px;
        no-frame: true;
        background: transparent;
    }
}

pub mod animation;
pub mod callbacks;
pub mod clipboard;
pub mod dummy_window;
pub mod focus;
pub mod popup;
pub mod sync;
pub mod tooltip;
pub mod tray;
pub mod ui;
pub mod platform;
pub mod window_effects;
pub mod window_position;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

fn main() {
    std::env::set_var("SLINT_BACKEND", "winit-skia");
    std::env::set_var("SLINT_STYLE", "fluent");
    std::env::set_var("ICU4X_DATA_DIR", "");

    const WINDOW_WIDTH: f32 = 280.0;
    const WINDOW_HEIGHT: f32 = 396.0;

    eprintln!("Starting PasteBridge...");

    let memory_monitor = paste_bridge_core::memory::MemoryMonitor::new();
    let initial_memory = memory_monitor.update();
    eprintln!("[memory] Initial memory: {}", paste_bridge_core::memory::MemoryMonitor::format_memory(initial_memory));

    // 启动时立即让 OS 回收空闲页面，减少内存占用
    if let Some(freed) = memory_monitor.trim_working_set() {
        eprintln!("[memory] Startup trim freed {}",
            paste_bridge_core::memory::MemoryMonitor::format_memory(freed));
    } else {
        eprintln!("[memory] Startup trim: not supported on this platform");
    }

    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let state = paste_bridge_core::state::AppState::new(&app_data_dir, usize::MAX);

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    // 从数据库加载持久化设置
    {
        // 加载主题设置
        if let Some(theme) = state.get_config("theme") {
            match theme.as_str() {
                "dark" => app.set_is_dark_mode(true),
                "light" => app.set_is_dark_mode(false),
                _ => {}
            }
            eprintln!("[config] Loaded theme: {}", theme);
        }

        // 加载窗口定位模式
        if let Some(mode) = state.get_config("window-position-mode") {
            if let Ok(mode_val) = mode.parse::<i32>() {
                app.set_window_position_mode(mode_val);
                eprintln!("[config] Loaded window-position-mode: {}", mode_val);
            }
        }
    }

    app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    let pos = window_position::calc_window_position(&app, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32);
    let _ = app.window().set_position(pos);
    let _ = app.window().show();
    tray::IS_VISIBLE.store(true, Ordering::SeqCst);

    focus::start_focus_tracker();

    let popup_tooltip = Arc::new(std::sync::Mutex::new(None::<PopupTooltipWindow>));
    let popup_tooltip_clone = popup_tooltip.clone();
    let popup_weak_holder: Arc<std::sync::Mutex<Option<slint::Weak<PopupTooltipWindow>>>> =
        Arc::new(std::sync::Mutex::new(None));
    let popup_weak_holder_clone = popup_weak_holder.clone();

    let clipboard_entries: Arc<std::sync::Mutex<Vec<sync::ClipboardEntry>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let clipboard_entries_clone = clipboard_entries.clone();

    let state_for_init = state.clone();
    let app_for_init = app.as_weak();
    let entries_for_init = clipboard_entries_clone.clone();
    slint::invoke_from_event_loop(move || {
        sync::sync_history_to_ui(&app_for_init, &state_for_init, &entries_for_init, false);
    }).ok();

    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    let entries_for_update = clipboard_entries_clone.clone();
    let memory_monitor_clone = Arc::new(memory_monitor);
    let mem_for_periodic = memory_monitor_clone.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(120));
        loop {
            if let Some(freed) = mem_for_periodic.trim_working_set() {
                eprintln!("[memory] Periodic trim freed {}",
                    paste_bridge_core::memory::MemoryMonitor::format_memory(freed));
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
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

            sync::sync_history_to_ui(&weak, &state, &entries_for_update, true);

            if let Some(before) = mem_before {
                let mem_after = mem.update();
                let mem_delta = if mem_after > before { mem_after - before } else { 0 };
                eprintln!("[memory] Update {}: {} (+{})",
                    count,
                    paste_bridge_core::memory::MemoryMonitor::format_memory(mem_after),
                    paste_bridge_core::memory::MemoryMonitor::format_memory(mem_delta));
            }
        });
    });

    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();

    #[cfg(target_os = "windows")]
    {
        animation::startup_window_fade_in();
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

    let mem_for_tray = memory_monitor_clone.clone();
    let handles = tray::setup_tray();
    let _tray_icon = handles.tray_icon;
    let weak_for_tray = app_weak.clone();
    tray::start_tray_event_loop(handles.show_id, handles.quit_id, hotkey_id, move || {
        let mem = mem_for_tray.clone();
        let _ = slint::invoke_from_event_loop({
            let weak = weak_for_tray.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    use slint::ComponentHandle;
                    let is_visible = tray::IS_VISIBLE.load(Ordering::SeqCst);
                    if is_visible {
                        animation::fade_window_out();

                        let _ = app.window().hide();
                        tray::IS_VISIBLE.store(false, Ordering::SeqCst);

                        let freed = mem.trim_working_set();
                        eprintln!("[memory] Tray: Window hidden, freed {}",
                            freed.map_or("N/A".to_string(), |b| paste_bridge_core::memory::MemoryMonitor::format_memory(b)));
                    } else {
                        app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
                        let pos = window_position::calc_window_position(&app, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32);
                        let _ = app.window().set_position(pos);

                        let _ = app.window().show();
                        tray::IS_VISIBLE.store(true, Ordering::SeqCst);

                        eprintln!("[memory] Tray: Window shown");

                        let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            unsafe {
                                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                            }
                            animation::fade_window_in();
                        }
                    }
                }
            }
        });
    });

    let callback_ctx = callbacks::CallbackContext {
        app_weak: app_weak.clone(),
        state: state.clone(),
        clipboard_entries: clipboard_entries_clone.clone(),
        memory_monitor: memory_monitor_clone.clone(),
        popup_tooltip: popup_tooltip_clone.clone(),
        popup_weak_holder: popup_weak_holder_clone.clone(),
    };
    callbacks::register_all(&app, &callback_ctx);

    let api_state = state.clone();
    std::thread::spawn(move || {
        let mut server = paste_bridge_core::api::ApiServer::new(18792);
        if let Err(e) = server.start(api_state) {
            eprintln!("[api] Server error: {}", e);
        }
    });

    tooltip::start_tooltip_zorder_monitor();

    eprintln!("About to run app...");

    popup::create_popup_tooltip(&popup_tooltip, &popup_weak_holder);

    dummy_window::create_and_run();
}
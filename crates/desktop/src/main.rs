slint::include_modules!();

// mimalloc: aggressively returns freed memory to the OS
// Rust's default allocator never unmaps pages, making Task Manager show high memory
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
pub mod drag_out;
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

    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let state = paste_bridge_core::state::AppState::new(&app_data_dir, usize::MAX);

    // 初始化默认收藏夹
    state.init_default_favorite_folders();

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

        // 加载焦点恢复设置
        if let Some(restore_focus) = state.get_config("restore-focus") {
            match restore_focus.as_str() {
                "true" | "1" => {
                    focus::set_restore_focus_enabled(true);
                    app.set_restore_focus_enabled(true);
                }
                "false" | "0" => {
                    focus::set_restore_focus_enabled(false);
                    app.set_restore_focus_enabled(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded restore-focus: {}", restore_focus);
        }

        // 加载鼠标悬停聚焦设置
        if let Some(mouse_hover_focus) = state.get_config("mouse-hover-focus") {
            match mouse_hover_focus.as_str() {
                "true" | "1" => {
                    focus::set_mouse_hover_focus_enabled(true);
                    app.set_mouse_hover_focus_enabled(true);
                }
                "false" | "0" => {
                    focus::set_mouse_hover_focus_enabled(false);
                    app.set_mouse_hover_focus_enabled(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded mouse-hover-focus: {}", mouse_hover_focus);
        }

        // 加载置顶（pin）设置
        if let Some(pinned) = state.get_config("pinned") {
            match pinned.as_str() {
                "true" | "1" => {
                    app.set_pinned(true);
                }
                "false" | "0" => {
                    app.set_pinned(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded pinned: {}", pinned);
        }

        // 加载窗口大小设置
        let mut loaded_width = WINDOW_WIDTH;
        let mut loaded_height = WINDOW_HEIGHT;

        if let Some(width_str) = state.get_config("window-width") {
            if let Ok(width) = width_str.parse::<f32>() {
                if width >= 200.0 && width <= 600.0 {
                    loaded_width = width;
                    eprintln!("[config] Loaded window-width: {}", width);
                }
            }
        }

        if let Some(height_str) = state.get_config("window-height") {
            if let Ok(height) = height_str.parse::<f32>() {
                if height >= 300.0 && height <= 800.0 {
                    loaded_height = height;
                    eprintln!("[config] Loaded window-height: {}", height);
                }
            }
        }

        app.window().set_size(slint::LogicalSize::new(loaded_width, loaded_height));
        let pos = window_position::calc_window_position(&app, loaded_width as i32, loaded_height as i32);
        let _ = app.window().set_position(pos);
    }
    let _ = app.window().show();
    tray::IS_VISIBLE.store(true, Ordering::SeqCst);

    focus::start_focus_tracker();

    // Window size change monitor
    {
        let app_weak_for_size = app_weak.clone();
        let last_width = Arc::new(std::sync::atomic::AtomicI32::new(app.window().size().width as i32));
        let last_height = Arc::new(std::sync::atomic::AtomicI32::new(app.window().size().height as i32));

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Some(app) = app_weak_for_size.upgrade() {
                    let current_size = app.window().size();
                    let current_width = current_size.width as i32;
                    let current_height = current_size.height as i32;

                    let last_w = last_width.load(std::sync::atomic::Ordering::Relaxed);
                    let last_h = last_height.load(std::sync::atomic::Ordering::Relaxed);

                    if current_width != last_w || current_height != last_h {
                        last_width.store(current_width, std::sync::atomic::Ordering::Relaxed);
                        last_height.store(current_height, std::sync::atomic::Ordering::Relaxed);

                        let app_clone = app_weak_for_size.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_clone.upgrade() {
                                app.set_size_tooltip_visible(true);
                                app.set_last_width(current_width);
                                app.set_last_height(current_height);
                            }
                        });
                    }
                } else {
                    break;
                }
            }
        });
    }

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
    let app_data_dir_for_init = app_data_dir.clone();
    slint::invoke_from_event_loop(move || {
        sync::sync_history_to_ui(&app_for_init, &state_for_init, &entries_for_init, &app_data_dir_for_init, false);
    }).ok();

    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    let entries_for_update = clipboard_entries_clone.clone();
    clipboard::start_clipboard_monitor(state_for_clipboard, {
        let app_data_dir_for_clip_cb = app_data_dir.clone();
        move || {
            let weak = app_weak_clone.clone();
            let state = state_for_ui.clone();
            let entries_for_update = entries_for_update.clone();

            sync::sync_history_to_ui_async(
                weak,
                state,
                entries_for_update,
                app_data_dir_for_clip_cb.clone(),
                true,
            );
        }
    }, app_data_dir.clone());

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
                        animation::fade_window_out();

                        let _ = app.window().hide();
                        tray::IS_VISIBLE.store(false, Ordering::SeqCst);
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
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                focus::restore_previous_focus();
                            });
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
        popup_tooltip: popup_tooltip_clone.clone(),
        popup_weak_holder: popup_weak_holder_clone.clone(),
        app_data_dir: Arc::new(app_data_dir.clone()),
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
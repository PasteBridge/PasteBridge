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
fn get_focused_caret_screen_pos() -> Option<(i32, i32)> {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
            GetWindowRect, GUITHREADINFO, GUI_CARETBLINKING,
        };

        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.is_invalid() {
            return None;
        }

        let thread_id = GetWindowThreadProcessId(fg_hwnd, None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        if GetGUIThreadInfo(thread_id, &mut info).is_ok() {
            if info.flags.contains(GUI_CARETBLINKING) && !info.hwndCaret.is_invalid() {
                let mut window_rect = std::mem::zeroed();
                if GetWindowRect(info.hwndCaret, &mut window_rect).is_ok() {
                    let screen_x = window_rect.left + info.rcCaret.left;
                    let screen_y = window_rect.top + info.rcCaret.top;
                    return Some((screen_x, screen_y));
                }
            }
        }
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn get_focused_caret_screen_pos() -> Option<(i32, i32)> {
    None
}

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

fn calc_window_position(app: &AppWindow, win_w: i32, win_h: i32) -> slint::PhysicalPosition {
    // 优先检测外部输入焦点，有焦点则定位在光标右侧 10px
    if let Some((caret_x, caret_y)) = get_focused_caret_screen_pos() {
        return slint::PhysicalPosition::new(caret_x + 10, caret_y);
    }

    // 无外部输入焦点，使用用户选择的模式
    let mode = app.get_window_position_mode();
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    match mode {
        0 => {
            let x = (screen_w - win_w) / 2;
            let y = (((screen_h as f64) * 0.618) - (win_h as f64) * 0.233) as i32;
            slint::PhysicalPosition::new(x, y)
        }
        _ => {
            let (cursor_x, cursor_y) = window_effects::get_cursor_pos();
            let x = cursor_x - win_w / 2;
            let y = cursor_y - win_h / 2;
            slint::PhysicalPosition::new(x, y)
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

    let state = paste_bridge_core::state::AppState::new(&app_data_dir, usize::MAX);

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    app.window().set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    let pos = calc_window_position(&app, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32);
    let _ = app.window().set_position(pos);
    tray::IS_VISIBLE.store(true, Ordering::SeqCst);

    let popup_tooltip = std::sync::Arc::new(std::sync::Mutex::new(None::<PopupTooltipWindow>));
    let popup_tooltip_clone = popup_tooltip.clone();
    let popup_weak_holder: std::sync::Arc<std::sync::Mutex<Option<slint::Weak<PopupTooltipWindow>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let popup_weak_holder_clone = popup_weak_holder.clone();

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
                        let pos = calc_window_position(&app, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32);
                        let _ = app.window().set_position(pos);
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
    let popup_for_show = popup_tooltip_clone.clone();
    let app_weak_for_pos = app_weak.clone();
    app.on_show_hover_tooltip_index(move |index: i32| {
        let idx = index as usize;
        let entries = entries_for_hover.lock().unwrap();
        if idx >= entries.len() {
            return;
        }
        let id = entries[idx].id;
        if let Some(item) = state_for_hover.get_item(id) {
            if let Some(text) = item.content_text {
                let popup_lock = popup_for_show.lock().unwrap();
                if let Some(ref popup) = *popup_lock {
                    popup.set_content_text(text.into());
                    
                    let timestamp_str = {
                        let unix_ts_secs = item.created_at / 1000;
                        let dt = chrono::DateTime::from_timestamp(unix_ts_secs, 0)
                            .unwrap_or_else(|| chrono::Utc::now());
                        let local = dt.with_timezone(&chrono::Local);
                        local.format("%Y-%m-%d %H:%M:%S").to_string()
                    };
                    popup.set_content_timestamp(timestamp_str.into());
                    
                    popup.set_show_pending(true);
                    popup.set_show_state(false);

                    #[cfg(target_os = "windows")]
                    {
                        use windows::Win32::UI::WindowsAndMessaging::*;
                        use windows::Win32::Foundation::*;
                        
                        unsafe {
                            let mut point = POINT { x: 0, y: 0 };
                            if GetCursorPos(&mut point).is_ok() {
                                let screen_w = GetSystemMetrics(SM_CXSCREEN);
                                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                                
                                popup.set_mouse_x(point.x as f32);
                                popup.set_mouse_y(point.y as f32);
                                popup.set_screen_width(screen_w as f32);
                                popup.set_screen_height(screen_h as f32);
                            }
                        }
                    }
                }
            }
        }
    });

    let popup_for_hide = popup_tooltip_clone.clone();
    let popup_weak_for_hide = popup_weak_holder_clone.clone();
    app.on_hide_hover_tooltip(move || {
        let popup_lock = popup_for_hide.lock().unwrap();
        if let Some(ref popup) = *popup_lock {
            popup.set_show_pending(false);
            popup.set_show_state(false);
        }
        // After fade-out animation (200ms), truly hide the window to prevent it from
        // intercepting mouse events on the main window
        let weak_guard = popup_weak_for_hide.lock().unwrap();
        if let Some(ref popup_weak) = *weak_guard {
            let weak_clone = popup_weak.clone();
            slint::Timer::single_shot(std::time::Duration::from_millis(250), move || {
                if let Some(p) = weak_clone.upgrade() {
                    let _ = p.hide();
                }
            });
        }
    });

    let entries_for_hover2 = clipboard_entries_clone.clone();
    let state_for_hover2 = state.clone();
    let popup_for_set = popup_tooltip_clone.clone();
    let app_weak_for_pos2 = app_weak.clone();
    app.on_set_hovered_index(move |index: i32| {
        if index < 0 {
            let popup_lock = popup_for_set.lock().unwrap();
            if let Some(ref popup) = *popup_lock {
                popup.set_show_pending(false);
                popup.set_show_state(false);
            }
        } else {
            let idx = index as usize;
            let entries = entries_for_hover2.lock().unwrap();
            if idx >= entries.len() {
                return;
            }
            let id = entries[idx].id;
            if let Some(item) = state_for_hover2.get_item(id) {
                if let Some(text) = item.content_text {
                    let popup_lock = popup_for_set.lock().unwrap();
                    if let Some(ref popup) = *popup_lock {
                        popup.set_content_text(text.into());
                        
                        let timestamp_str = {
                            let unix_ts_secs = item.created_at / 1000;
                            let dt = chrono::DateTime::from_timestamp(unix_ts_secs, 0)
                                .unwrap_or_else(|| chrono::Utc::now());
                            let local = dt.with_timezone(&chrono::Local);
                            local.format("%Y-%m-%d %H:%M:%S").to_string()
                        };
                        popup.set_content_timestamp(timestamp_str.into());
                        
                        popup.set_show_pending(true);
                        popup.set_show_state(false);
                        
                        #[cfg(target_os = "windows")]
                        {
                            use windows::Win32::UI::WindowsAndMessaging::*;
                            use windows::Win32::Foundation::*;
                            
                            unsafe {
                                let mut point = POINT { x: 0, y: 0 };
                                if GetCursorPos(&mut point).is_ok() {
                                    let tooltip_w = 200;
                                    let tooltip_h = 380;
                                    
                                    let screen_w = GetSystemMetrics(SM_CXSCREEN);
                                    let screen_h = GetSystemMetrics(SM_CYSCREEN);
                                    
                                    let mut popup_x = point.x;
                                    let mut popup_y = point.y;
                                    
                                    if popup_x + tooltip_w > screen_w {
                                        popup_x = point.x - tooltip_w;
                                    }
                                    if popup_x < 0 {
                                        popup_x = 0;
                                    }
                                    
                                    if popup_y + tooltip_h > screen_h {
                                        popup_y = point.y - tooltip_h;
                                    }
                                    if popup_y < 0 {
                                        popup_y = 0;
                                    }
                                    
                                    popup.window().set_position(slint::PhysicalPosition::new(popup_x, popup_y));
                                }
                            }
                        }
                    }
                }
            }
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

    let app_for_share = app.as_weak();
    app.on_toggle_share(move || {
        if let Some(app) = app_for_share.upgrade() {
            use slint::ComponentHandle;
            let current = app.get_share_visible();
            app.set_share_visible(!current);
        }
    });

    let state_for_mock = state.clone();
    let app_for_mock = app.as_weak();
    let entries_for_mock = clipboard_entries_clone.clone();
    app.on_add_mock_data(move || {
        let inserted = state_for_mock.add_mock_data(100);
        eprintln!("Added {} mock data entries", inserted);
        
        let app_clone = app_for_mock.clone();
        let entries_for_update = entries_for_mock.clone();
        let state_clone = state_for_mock.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = app_clone.upgrade() {
                let history = state_clone.get_history();
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
            }
        });
    });

    let api_state = state.clone();
    std::thread::spawn(move || {
        let mut server = paste_bridge_core::api::ApiServer::new(18792);
        if let Err(e) = server.start(api_state) {
            eprintln!("[api] Server error: {}", e);
        }
    });

    tooltip::start_tooltip_zorder_monitor();

    eprintln!("About to run app...");

    {
        let mut popup_guard = popup_tooltip.lock().unwrap();
        if popup_guard.is_none() {
            let popup_win = PopupTooltipWindow::new().unwrap();
            
            popup_win.on_hide_window({
                let popup_win_weak = popup_win.as_weak();
                move || {
                    if let Some(p) = popup_win_weak.upgrade() {
                        p.hide().unwrap();
                    }
                }
            });
            popup_win.on_on_delay_show({
                let popup_win_weak = popup_win.as_weak();
                move || {
                    if let Some(popup) = popup_win_weak.upgrade() {
                        popup.set_show_state(true);
                        popup.show().unwrap();
                    }
                }
            });
            
            popup_win.window().set_size(slint::LogicalSize::new(200.0, 200.0));
            popup_win.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
            popup_win.hide().unwrap();
            *popup_weak_holder.lock().unwrap() = Some(popup_win.as_weak());
            *popup_guard = Some(popup_win);
            eprintln!("[popup] Tooltip popup window created");
            
            #[cfg(target_os = "windows")]
            {
                use std::thread;
                use std::time::Duration;
                use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, EnumWindows, GetWindowTextW, GetClassNameW};
                use windows::Win32::Foundation::{HWND, BOOL, LPARAM};
                use windows::core::PCWSTR;

                thread::spawn(move || {
                    let mut found_hwnd = HWND::default();
                    
                    static mut FOUND_HWND: HWND = HWND(std::ptr::null_mut());
                    
                    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
                        let mut class_buf = [0u16; 256];
                        let class_len = GetClassNameW(hwnd, &mut class_buf);
                        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
                        
                        // Slint windows usually have a specific class name
                        if class_name.contains("Slint") || class_name.contains("winit") {
                            let mut title_buf = [0u16; 256];
                            let title_len = GetWindowTextW(hwnd, &mut title_buf);
                            let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
                            
                            // Tooltip window should have empty title
                            if title.is_empty() {
                                FOUND_HWND = hwnd;
                                return BOOL(0);
                            }
                        }
                        
                        BOOL(1)
                    }
                    
                    for _ in 0..50 {
                        FOUND_HWND = HWND::default();
                        
                        // Try to find the tooltip window
                        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(0));
                        found_hwnd = FOUND_HWND;
                        
                        if !found_hwnd.is_invalid() {
                            // Verify it's not the main window
                            let app_hwnd = crate::window_effects::APP_HWND.load(std::sync::atomic::Ordering::SeqCst);
                            if found_hwnd.0 as isize != app_hwnd {
                                break;
                            }
                        }
                        
                        thread::sleep(Duration::from_millis(20));
                    }
                    
                    if !found_hwnd.is_invalid() {
                        crate::tooltip::set_tooltip_hwnd(found_hwnd.0 as isize);
                        eprintln!("[popup] Tooltip HWND found: {:?}", found_hwnd);
                    } else {
                        eprintln!("[popup] Failed to find tooltip HWND");
                    }
                });
            }
        }
    }

    app.run().ok();
    eprintln!("App run() finished");
}
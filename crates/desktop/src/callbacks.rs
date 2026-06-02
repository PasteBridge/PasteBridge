use std::sync::Arc;
use slint::ComponentHandle;
use crate::sync::ClipboardEntry;
use crate::AppWindow;

pub struct CallbackContext {
    pub app_weak: slint::Weak<AppWindow>,
    pub state: Arc<paste_bridge_core::state::AppState>,
    pub clipboard_entries: Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    pub memory_monitor: Arc<paste_bridge_core::memory::MemoryMonitor>,
    pub popup_tooltip: Arc<std::sync::Mutex<Option<crate::PopupTooltipWindow>>>,
    pub popup_weak_holder: Arc<std::sync::Mutex<Option<slint::Weak<crate::PopupTooltipWindow>>>>,
}

pub fn register_all(app: &AppWindow, ctx: &CallbackContext) {
    register_save_setting(app, ctx);
    register_start_drag(app, ctx);
    register_hide_window(app, ctx);
    register_copy_item(app, ctx);
    register_show_hover_tooltip(app, ctx);
    register_hide_hover_tooltip(app, ctx);
    register_clear_history(app, ctx);
    register_minimize_window(app, ctx);
    register_quit_app(app);
    register_toggle_settings(app, ctx);
    register_toggle_share(app, ctx);
    register_toggle_sort_order(app, ctx);
    register_add_mock_data(app, ctx);
}

fn register_save_setting(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    app.on_save_setting(move |key: slint::SharedString, value: slint::SharedString| {
        let saved = state.set_config(&key, &value);
        eprintln!("[config] Save setting: {} = {} (saved: {})", key, value, saved);
    });
}

fn register_start_drag(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_start_drag(move || {
        if let Some(_w) = weak.upgrade() {
            #[cfg(target_os = "windows")]
            {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                    let hwnd = GetForegroundWindow();
                    crate::window_position::window_util::start_drag(hwnd.0);
                }
            }
        }
    });
}

fn register_hide_window(app: &AppWindow, ctx: &CallbackContext) {
    use std::sync::atomic::Ordering;
    let weak = ctx.app_weak.clone();
    let mem = ctx.memory_monitor.clone();
    app.on_hide_window(move || {
        if let Some(app) = weak.upgrade() {
            use slint::ComponentHandle;
            crate::animation::fade_window_out();
            let _ = app.window().hide();
            crate::tray::IS_VISIBLE.store(false, Ordering::SeqCst);
            let freed = mem.trim_working_set();
            eprintln!("[memory] Window hidden, freed {}",
                freed.map_or("N/A".to_string(), |b| paste_bridge_core::memory::MemoryMonitor::format_memory(b)));
        }
    });
}

fn register_copy_item(app: &AppWindow, ctx: &CallbackContext) {
    let entries = ctx.clipboard_entries.clone();
    let state = ctx.state.clone();
    app.on_copy_item(move |index: i32| {
        let idx = index as usize;
        let entries = entries.lock().unwrap();
        if idx >= entries.len() {
            eprintln!("copy-item: index out of range: {}", idx);
            return;
        }
        let id = entries[idx].id;
        if let Some(item) = state.get_item(id) {
            if let Some(text) = item.content_text {
                paste_bridge_core::clipboard::set_clipboard_text(text.clone());
                eprintln!("Copied to clipboard (id={}): {}", id, text.chars().take(20).collect::<String>());
                #[cfg(target_os = "windows")]
                {
                    let pos = crate::tooltip::get_cursor_pos();
                    crate::tooltip::show_tooltip_at(pos.0, pos.1, "Copied");
                }
            } else {
                eprintln!("copy-item: item id {} has no text", id);
            }
        } else {
            eprintln!("copy-item: no item found for id {}", id);
        }
    });
}

fn register_show_hover_tooltip(app: &AppWindow, ctx: &CallbackContext) {
    let entries = ctx.clipboard_entries.clone();
    let state = ctx.state.clone();
    let popup = ctx.popup_tooltip.clone();
    app.on_show_hover_tooltip_index(move |index: i32| {
        let idx = index as usize;
        let entries = entries.lock().unwrap();
        if idx >= entries.len() {
            return;
        }
        let id = entries[idx].id;
        if let Some(item) = state.get_item(id) {
            if let Some(text) = item.content_text {
                let popup_lock = popup.lock().unwrap();
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
                    crate::tooltip::bring_tooltip_to_front();
                }
            }
        }
    });
}

fn register_hide_hover_tooltip(app: &AppWindow, ctx: &CallbackContext) {
    let popup = ctx.popup_tooltip.clone();
    let weak_holder = ctx.popup_weak_holder.clone();
    app.on_hide_hover_tooltip(move || {
        let popup_lock = popup.lock().unwrap();
        if let Some(ref popup) = *popup_lock {
            popup.set_show_pending(false);
            popup.set_show_state(false);
        }
        let weak_guard = weak_holder.lock().unwrap();
        if let Some(ref popup_weak) = *weak_guard {
            let weak_clone = popup_weak.clone();
            slint::Timer::single_shot(std::time::Duration::from_millis(250), move || {
                if let Some(p) = weak_clone.upgrade() {
                    let _ = p.hide();
                }
            });
        }
    });
}

fn register_clear_history(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_clear_history(move || {
        state.clear_history();
        let app_clone = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = app_clone.upgrade() {
                let model = std::rc::Rc::new(slint::VecModel::<slint::SharedString>::from(vec![]));
                w.set_clipboard_history(model.into());
                crate::animation::trigger_content_update_fade(w.as_weak());
            }
        });
        eprintln!("Clipboard history cleared");
    });
}

fn register_minimize_window(app: &AppWindow, ctx: &CallbackContext) {
    use std::sync::atomic::Ordering;
    let weak = ctx.app_weak.clone();
    let mem = ctx.memory_monitor.clone();
    app.on_minimize_window(move || {
        if let Some(app) = weak.upgrade() {
            use slint::ComponentHandle;
            crate::animation::fade_window_out();
            let _ = app.window().hide();
            crate::tray::IS_VISIBLE.store(false, Ordering::SeqCst);
            let freed = mem.trim_working_set();
            eprintln!("[memory] Window minimized (hidden), freed {}",
                freed.map_or("N/A".to_string(), |b| paste_bridge_core::memory::MemoryMonitor::format_memory(b)));
        }
    });
}

fn register_quit_app(app: &AppWindow) {
    app.on_quit_app(|| {
        eprintln!("Quit requested, exiting application...");
        std::process::exit(0);
    });
}

fn register_toggle_settings(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_toggle_settings(move || {
        if let Some(app) = weak.upgrade() {
            use slint::ComponentHandle;
            let current = app.get_settings_visible();
            app.set_settings_visible(!current);
            if current {
                app.window().request_redraw();
            }
        }
    });
}

fn register_toggle_share(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_toggle_share(move || {
        if let Some(app) = weak.upgrade() {
            let current = app.get_share_visible();
            app.set_share_visible(!current);
        }
    });
}

fn register_toggle_sort_order(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let entries = ctx.clipboard_entries.clone();
    app.on_toggle_sort_order(move || {
        if let Some(app) = weak.upgrade() {
            let new_ascending = !app.get_sort_ascending();
            app.set_sort_ascending(new_ascending);
            eprintln!("[sort] Toggled sort order: ascending = {}", new_ascending);
            crate::sync::sync_history_to_ui(&weak, &state, &entries, false);
        }
    });
}

fn register_add_mock_data(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let entries = ctx.clipboard_entries.clone();
    app.on_add_mock_data(move || {
        let inserted = state.add_mock_data(100);
        eprintln!("Added {} mock data entries", inserted);
        let weak = weak.clone();
        let entries = entries.clone();
        let state = state.clone();
        let _ = slint::invoke_from_event_loop(move || {
            crate::sync::sync_history_to_ui(&weak, &state, &entries, true);
        });
    });
}
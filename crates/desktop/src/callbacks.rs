use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use slint::{ComponentHandle, Model};
use crate::sync::ClipboardEntry;
use crate::AppWindow;

/// 防重入: 当前正在复制的条目 id,-1 表示空闲。
/// 防止 Slint 模型更新时 clicked 事件重复触发导致同一张图片被复制多次。
static COPYING_ITEM_ID: AtomicI64 = AtomicI64::new(-1);

pub struct CallbackContext {
    pub app_weak: slint::Weak<AppWindow>,
    pub state: Arc<paste_bridge_core::state::AppState>,
    pub clipboard_entries: Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    pub popup_tooltip: Arc<std::sync::Mutex<Option<crate::PopupTooltipWindow>>>,
    pub popup_weak_holder: Arc<std::sync::Mutex<Option<slint::Weak<crate::PopupTooltipWindow>>>>,
    pub app_data_dir: Arc<std::path::PathBuf>,
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
    register_restore_focus_setting(app);
    register_mouse_hover_focus_setting(app);
    register_reorder_favorite(app, ctx);
    register_drag_over_favorite_button(app, ctx);
    register_end_drag_to_favorite(app, ctx);
    register_cancel_drag_to_favorite(app, ctx);
    register_add_to_favorite(app, ctx);
    register_load_favorite_folder(app, ctx);
    register_back_to_all_history(app, ctx);
    register_create_favorite_folder(app, ctx);
    register_delete_favorite_folder(app, ctx);
    register_drag_out_started(app, ctx);
    register_toggle_pin(app, ctx);
    register_toggle_multi_select(app, ctx);
    register_toggle_selection(app, ctx);
    register_clear_multi_select(app, ctx);
    register_search_history(app, ctx);
    register_reset_window_size(app, ctx);
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
    app.on_hide_window(move || {
        if let Some(app) = weak.upgrade() {
            use slint::ComponentHandle;
            crate::animation::fade_window_out();
            let _ = app.window().hide();
            crate::tray::IS_VISIBLE.store(false, Ordering::SeqCst);
        }
    });
}

fn register_copy_item(app: &AppWindow, ctx: &CallbackContext) {
    let entries = ctx.clipboard_entries.clone();
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let app_data_dir = ctx.app_data_dir.clone();
    app.on_copy_item(move |index: i32| {
        let idx = index as usize;
        let entries = entries.lock().unwrap();
        if idx >= entries.len() {
            eprintln!("copy-item: index out of range: {}", idx);
            return;
        }
        let id = entries[idx].id;
        let is_image = matches!(entries[idx].content_type, paste_bridge_core::models::ContentType::Image);
        let image_path = entries[idx].image_path.clone();
        drop(entries);

        // 防重入: 同一 id 正在复制中,跳过此次重复触发
        if COPYING_ITEM_ID.compare_exchange(-1, id, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            eprintln!("copy-item: id={} still copying, skip redundant call", id);
            return;
        }

        if is_image {
            // ── 图片复制 ──
            let Some(rel_path) = image_path else {
                COPYING_ITEM_ID.store(-1, Ordering::SeqCst);
                eprintln!("copy-item: image id={} has no path", id);
                return;
            };
            let abs_path = app_data_dir.join(&rel_path);
            eprintln!("copy-item: image id={}, path={}", id, abs_path.display());

            // 后台线程写入剪贴板,不阻塞 UI 事件循环
            let weak_toast = weak.clone();
            std::thread::spawn(move || {
                let res = crate::clipboard::set_clipboard_image_blocking(&abs_path);
                match &res {
                    Ok((w, h)) => {
                        eprintln!("copy-item: 剪贴板图片就绪 ({}x{})", w, h);
                        // 标记监听线程跳过下一次图片检测,避免重复编码大图
                        crate::clipboard::skip_next_image_detect();
                    }
                    Err(e) => eprintln!("copy-item: 剪贴板图片写入失败: {}", e),
                }
                COPYING_ITEM_ID.store(-1, Ordering::SeqCst);

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = weak_toast.upgrade() {
                        app.set_toast_message(slint::SharedString::from("图片已复制"));
                        app.set_toast_visible(true);
                        let weak_toast2 = weak_toast.clone();
                        slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                            if let Some(a) = weak_toast2.upgrade() {
                                a.set_toast_visible(false);
                            }
                        });
                    }
                });
            });
            return;
        }

        // ── 文本复制 ──
        let Some(item) = state.get_item(id) else {
            COPYING_ITEM_ID.store(-1, Ordering::SeqCst);
            eprintln!("copy-item: no item found for id {}", id);
            return;
        };
        let Some(text) = item.content_text else {
            COPYING_ITEM_ID.store(-1, Ordering::SeqCst);
            eprintln!("copy-item: item id {} has no text", id);
            return;
        };
        // 后台线程写入剪贴板,不阻塞 UI 事件循环
        let weak_toast = weak.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::clipboard::set_clipboard_text_blocking(text.clone()) {
                eprintln!("copy-item: 剪贴板文本写入失败: {}", e);
            } else {
                eprintln!("Copied to clipboard (id={}): {}",
                    id, text.chars().take(20).collect::<String>());
                // 标记监听线程跳过下一次文本检测,避免触发全量历史刷新和图片重加载
                crate::clipboard::skip_next_text_detect();
            }
            COPYING_ITEM_ID.store(-1, Ordering::SeqCst);

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = weak_toast.upgrade() {
                    app.set_toast_message(slint::SharedString::from("Copied"));
                    app.set_toast_visible(true);
                    let weak_toast2 = weak_toast.clone();
                    slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                        if let Some(a) = weak_toast2.upgrade() {
                            a.set_toast_visible(false);
                        }
                    });
                }
            });
        });
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
                        use chrono::{DateTime, Local, TimeZone};
                        let ts = match Local.timestamp_millis_opt(item.created_at).single() {
                            Some(dt) => {
                                let now: DateTime<Local> = Local::now();
                                let diff = now.signed_duration_since(dt);

                                if diff.num_minutes() < 1 {
                                    "刚刚".into()
                                } else if diff.num_minutes() < 60 {
                                    format!("{}分钟前", diff.num_minutes())
                                } else if diff.num_hours() < 24 {
                                    format!("{}小时前", diff.num_hours())
                                } else if diff.num_days() < 30 {
                                    format!("{}天前", diff.num_days())
                                } else {
                                    dt.format("%m-%d").to_string()
                                }
                            }
                            None => String::new(),
                        };
                        ts
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
                let ts_model = std::rc::Rc::new(slint::VecModel::<slint::SharedString>::from(vec![]));
                w.set_clipboard_timestamps(ts_model.into());
                crate::animation::trigger_content_update_fade(w.as_weak());
            }
        });
        eprintln!("Clipboard history cleared");
    });
}

fn register_minimize_window(app: &AppWindow, ctx: &CallbackContext) {
    use std::sync::atomic::Ordering;
    let weak = ctx.app_weak.clone();
    app.on_minimize_window(move || {
        if let Some(app) = weak.upgrade() {
            use slint::ComponentHandle;
            crate::animation::fade_window_out();
            let _ = app.window().hide();
            crate::tray::IS_VISIBLE.store(false, Ordering::SeqCst);
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
            if !current {
                app.set_settings_flash(true);
                let w = app.as_weak();
                slint::Timer::single_shot(std::time::Duration::from_millis(0), move || {
                    if let Some(w) = w.upgrade() {
                        w.set_settings_flash(false);
                        w.set_settings_visible(true);
                    }
                });
            } else {
                app.set__settings_closing(true);
                app.set_settings_visible(false);
            }
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
            if !current {
                app.set_share_flash(true);
                let w = app.as_weak();
                slint::Timer::single_shot(std::time::Duration::from_millis(0), move || {
                    if let Some(w) = w.upgrade() {
                        w.set_share_flash(false);
                        w.set_share_visible(true);
                    }
                });
            } else {
                app.set__share_closing(true);
                app.set_share_visible(false);
            }
        }
    });
}

fn register_toggle_sort_order(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let entries = ctx.clipboard_entries.clone();
    let app_data_dir = ctx.app_data_dir.clone();
    app.on_toggle_sort_order(move || {
        if let Some(app) = weak.upgrade() {
            let new_ascending = !app.get_sort_ascending();
            app.set_sort_ascending(new_ascending);
            eprintln!("[sort] Toggled sort order: ascending = {}", new_ascending);
            crate::sync::sync_history_to_ui(&weak, &state, &entries, &app_data_dir, false);
        }
    });
}

fn register_add_mock_data(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let entries = ctx.clipboard_entries.clone();
    let app_data_dir = ctx.app_data_dir.clone();
    app.on_add_mock_data(move || {
        let inserted = state.add_mock_data(100);
        eprintln!("Added {} mock data entries", inserted);
        let weak = weak.clone();
        let entries = entries.clone();
        let state = state.clone();
        let app_data_dir = app_data_dir.clone();
        let _ = slint::invoke_from_event_loop(move || {
            crate::sync::sync_history_to_ui(&weak, &state, &entries, &app_data_dir, true);
        });
    });
}

fn register_restore_focus_setting(app: &AppWindow) {
    app.on_restore_focus_enabled_change(move |enabled| {
        crate::focus::set_restore_focus_enabled(enabled);
        eprintln!("[focus] Restore focus setting changed to: {} (from UI)", enabled);
    });
}

fn register_mouse_hover_focus_setting(app: &AppWindow) {
    app.on_mouse_hover_focus_enabled_change(move |enabled| {
        crate::focus::set_mouse_hover_focus_enabled(enabled);
        eprintln!("[focus] Mouse hover focus setting changed to: {} (from UI)", enabled);
    });
}

fn register_reorder_favorite(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_reorder_favorite(move |old_index: i32, new_index: i32| {
        eprintln!("[favorite] Reorder: {} -> {}", old_index, new_index);
        let old = old_index as usize;
        let new = new_index as usize;

        if let Some(app) = weak.upgrade() {
            // Reorder the current Slint model directly
            let current_items = app.get_favorite_items();
            let count = current_items.row_count();
            if old < count && new <= count {
                let mut items: Vec<crate::FavoriteItem> = (0..count)
                    .filter_map(|i| current_items.row_data(i))
                    .collect();
                if old < items.len() {
                    let moved = items.remove(old);
                    let insert_at = if new > old { new - 1 } else { new };
                    items.insert(insert_at, moved);
                    let model = std::rc::Rc::new(slint::VecModel::from(items));
                    let _ = app.set_favorite_items(model.into());
                }
            }
        }

        // Also persist to database
        state.reorder_favorite_folders(old, new);
    });
}

fn register_drag_over_favorite_button(app: &AppWindow, _ctx: &CallbackContext) {
    let weak = _ctx.app_weak.clone();
    app.on_drag_over_favorite_button(move || {
        if let Some(app) = weak.upgrade() {
            // 清除拖拽状态和视觉状态
            app.set_is_dragging_to_favorite(false);
            app.set_is_dragging(false);
            // 显示收藏夹面板
            if !app.get_favorite_list_visible() {
                app.set_favorite_list_visible(true);
            }
            eprintln!("[drag] Dropped on favorite button, showing favorite list");
        }
    });
}

fn register_end_drag_to_favorite(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_end_drag_to_favorite(move || {
        if let Some(_app) = weak.upgrade() {
            // Keep the favorite list open - user needs to click a folder
            // The reset will happen in add-to-favorite callback
            eprintln!("[drag] End drag to favorite");
        }
    });
}

fn register_cancel_drag_to_favorite(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_cancel_drag_to_favorite(move || {
        if let Some(app) = weak.upgrade() {
            // 重置拖拽状态和视觉状态
            app.set_is_dragging_to_favorite(false);
            app.set_is_dragging(false);
            app.set_dragging_text(slint::SharedString::from(""));
            // 递增 hover 刷新计数器，强制 Slint 重新评估所有 item 的 hover 状态
            let current = app.get_hover_refresh_tick();
            app.set_hover_refresh_tick(current + 1);
            eprintln!("[drag] Cancel drag to favorite");
        }
    });
}

fn register_add_to_favorite(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_add_to_favorite(move |favorite_index: i32, text: slint::SharedString| {
        let idx = favorite_index as usize;
        let text_str = text.to_string();

        if text_str.is_empty() {
            eprintln!("[favorite] Add to favorite failed: empty text");
            return;
        }

        eprintln!("[favorite] Adding to favorite folder {}: text_len={}", 
            idx, text_str.len());

        // 多选模式：批量添加
        if let Some(app) = weak.upgrade() {
            if app.get_multi_select_mode() {
                let multi = app.get_multi_select_texts();
                let multi_count = multi.row_count();
                if multi_count > 0 {
                    eprintln!("[favorite] Multi-select add: {} items to folder {}", multi_count, idx);
                    let mut added = 0;
                    for i in 0..multi_count {
                        if let Some(t) = multi.row_data(i) {
                            let t_str = t.to_string();
                            if t_str.is_empty() { continue; }
                            let item_id = state.add_item(&t_str);
                            if state.add_item_to_favorite_folder(idx, item_id) {
                                added += 1;
                            }
                        }
                    }
                    eprintln!("[favorite] Multi-select added {} items", added);

                    // Refresh the favorite items list
                    let current_items = app.get_favorite_items();
                    let count = current_items.row_count();
                    if idx < count {
                        let mut items: Vec<crate::FavoriteItem> = (0..count)
                            .filter_map(|i| current_items.row_data(i))
                            .collect();
                        if idx < items.len() {
                            items[idx].count += added;
                            let model = std::rc::Rc::new(slint::VecModel::from(items));
                            let _ = app.set_favorite_items(model.into());
                        }
                    }

                    // Reset drag state and clear multi-select
                    app.set_is_dragging_to_favorite(false);
                    app.set_is_dragging(false);
                    app.set_dragging_text(slint::SharedString::from(""));
                    app.set_mouse_window_x(0.0f32.into());
                    app.set_mouse_window_y(0.0f32.into());
                    app.set_favorite_list_visible(false);

                    // Exit multi-select mode
                    app.set_multi_select_mode(false);
                    let empty_states = std::rc::Rc::new(slint::VecModel::from(Vec::<bool>::new()));
                    let _ = app.set_selected_states(empty_states.into());
                    let empty_texts = std::rc::Rc::new(slint::VecModel::from(Vec::<slint::SharedString>::new()));
                    let _ = app.set_multi_select_texts(empty_texts.into());
                    return;
                }
            }
        }

        // 单选模式：单条添加
        let item_id = state.add_item(&text_str);
        let actually_added = state.add_item_to_favorite_folder(idx, item_id);

        // Refresh the favorite items list
        if let Some(app) = weak.upgrade() {
            let current_items = app.get_favorite_items();
            let count = current_items.row_count();
            if idx < count {
                let mut items: Vec<crate::FavoriteItem> = (0..count)
                    .filter_map(|i| current_items.row_data(i))
                    .collect();
                if idx < items.len() {
                    // 重复 (folder, item) 会被 UNIQUE 约束 + INSERT OR IGNORE 拦截,
                    // 此时 actually_added=false,不能 +1,否则显示值会偏离 DB 真实值。
                    if actually_added {
                        items[idx].count += 1;
                    }
                    let model = std::rc::Rc::new(slint::VecModel::from(items));
                    let _ = app.set_favorite_items(model.into());
                }
            }

            // Reset drag state and close favorite list
            app.set_is_dragging_to_favorite(false);
            app.set_is_dragging(false);
            app.set_dragging_text(slint::SharedString::from(""));
            app.set_mouse_window_x(0.0f32.into());
            app.set_mouse_window_y(0.0f32.into());
            app.set_favorite_list_visible(false);

            eprintln!("[favorite] Successfully added to favorite folder {}", idx);
        }
    });
}

fn register_load_favorite_folder(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_load_favorite_folder(move |folder_index: i32| {
        let idx = folder_index as usize;

        if let Some(app) = weak.upgrade() {
            // 获取文件夹名称
            let items = app.get_favorite_items();
            let count = items.row_count();
            let folder_name = if idx < count {
                items.row_data(idx).map(|f| f.name).unwrap_or_default()
            } else {
                slint::SharedString::new()
            };

            // 加载文件夹内容
            let item_list = state.get_favorite_folder_items(idx);
            let texts: Vec<slint::SharedString> = item_list.iter()
                .filter_map(|item| item.content_text.clone().map(|s| s.into()))
                .collect();

            let model = std::rc::Rc::new(slint::VecModel::from(texts.clone()));
            let _ = app.set_favorite_folder_contents(model.into());

            // 同步构建 text-only entries,供统一列表渲染
            let folder_entries: Vec<crate::ClipboardEntryData> = item_list.iter().filter_map(|item| {
                let text = item.content_text.clone()?;
                Some(crate::ClipboardEntryData {
                    id: item.id as i32,
                    content_kind: crate::ContentKind::Text,
                    text: text.into(),
                    image: slint::Image::default(),
                    timestamp: slint::SharedString::new(),
                    width: 0,
                    height: 0,
                    file_size: 0,
                })
            }).collect();
            let entries_model = std::rc::Rc::new(slint::VecModel::from(folder_entries));
            let _ = app.set_favorite_folder_entries(entries_model.into());

            app.set_current_favorite_folder(idx as i32);
            app.set_current_favorite_folder_name(folder_name);
            app.set_favorite_list_visible(false);

            eprintln!("[favorite] Loaded folder {}: {} items", idx, texts.len());
        }
    });
}

fn register_back_to_all_history(app: &AppWindow, _ctx: &CallbackContext) {
    let weak = _ctx.app_weak.clone();
    app.on_back_to_all_history(move || {
        if let Some(app) = weak.upgrade() {
            app.set_current_favorite_folder(-1);
            app.set_current_favorite_folder_name(slint::SharedString::new());
            eprintln!("[favorite] Back to all history");
        }
    });
}

fn register_create_favorite_folder(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_create_favorite_folder(move |name: slint::SharedString| {
        let name_str = name.to_string().trim().to_string();
        if name_str.is_empty() {
            eprintln!("[favorite] Create folder failed: empty name");
            return;
        }

        if let Some(app) = weak.upgrade() {
            state.insert_favorite_folder(&name_str);

            // Refresh the favorite items model from database
            let folders = state.get_favorite_folders();
            let items: Vec<crate::FavoriteItem> = folders.iter().map(|f| {
                let count = state.get_favorite_folder_item_count(f.id) as i32;
                crate::FavoriteItem {
                    id: f.id as i32,
                    name: f.name.clone().into(),
                    count,
                }
            }).collect();
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            let _ = app.set_favorite_items(model.into());
            eprintln!("[favorite] Created folder: {}", name_str);
        }
    });
}

fn register_delete_favorite_folder(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_delete_favorite_folder(move |folder_index: i32| {
        let idx = folder_index as usize;

        if let Some(app) = weak.upgrade() {
            let folders = state.get_favorite_folders();
            if idx < folders.len() {
                let folder_id = folders[idx].id;
                let folder_name = folders[idx].name.clone();
                state.delete_favorite_folder(folder_id);

                // Refresh the favorite items model from database
                let updated_folders = state.get_favorite_folders();
                let items: Vec<crate::FavoriteItem> = updated_folders.iter().map(|f| {
                    let count = state.get_favorite_folder_item_count(f.id) as i32;
                    crate::FavoriteItem {
                        id: f.id as i32,
                        name: f.name.clone().into(),
                        count,
                    }
                }).collect();
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                let _ = app.set_favorite_items(model.into());
                eprintln!("[favorite] Deleted folder: {}", folder_name);
            }
        }
    });
}

fn register_drag_out_started(app: &AppWindow, ctx: &CallbackContext) {
    let entries = ctx.clipboard_entries.clone();
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_drag_out_started(move |index: i32| {
        let idx = index as usize;

        // Get the text from clipboard entries
        let text = {
            let entries_guard = entries.lock().unwrap();
            if idx >= entries_guard.len() {
                eprintln!("[drag-out] Index out of range: {}", idx);
                return;
            }
            let id = entries_guard[idx].id;
            drop(entries_guard);

            if let Some(item) = state.get_item(id) {
                item.content_text.clone()
            } else {
                eprintln!("[drag-out] No item found for id {}", id);
                None
            }
        };

        if let Some(text) = text {
            let text_len = text.len();
            eprintln!("[drag-out] Started drag for text: {} chars", text_len);

            // 多选模式：收集所有选中项的文本
            let multi_texts: Vec<String> = {
                let mut texts = Vec::new();
                if let Some(app) = weak.upgrade() {
                    if app.get_multi_select_mode() {
                        let selected = app.get_selected_states();
                        let count = selected.row_count();
                        let entries_guard = entries.lock().unwrap();
                        for i in 0..count {
                            if selected.row_data(i).unwrap_or(false) {
                                if i < entries_guard.len() {
                                    let id = entries_guard[i].id;
                                    if let Some(item) = state.get_item(id) {
                                        if let Some(content) = item.content_text {
                                            texts.push(content);
                                        }
                                    }
                                }
                            }
                        }
                        drop(entries_guard);

                        if !texts.is_empty() {
                            let shared: Vec<slint::SharedString> = texts.iter()
                                .map(|t| slint::SharedString::from(t.clone()))
                                .collect();
                            let model = std::rc::Rc::new(slint::VecModel::from(shared));
                            let _ = app.set_multi_select_texts(model.into());
                            eprintln!("[drag-out] Multi-select: collected {} texts", texts.len());
                        }
                    }
                }
                texts
            };

            // ── 获取根窗口句柄 ──
            #[cfg(target_os = "windows")]
            let root_hwnd: isize = {
                #[link(name = "user32")]
                extern "system" {
                    fn GetForegroundWindow() -> isize;
                    fn GetAncestor(hWnd: isize, gaFlags: u32) -> isize;
                }
                const GA_ROOT: u32 = 2;
                let fg = unsafe { GetForegroundWindow() };
                let root = unsafe { GetAncestor(fg, GA_ROOT) };
                eprintln!("[drag-out] hwnd: 0x{:x}, root: 0x{:x}", fg, root);
                root
            };
            #[cfg(not(target_os = "windows"))]
            let root_hwnd: isize = 0;

            // 设置拖拽状态
            if let Some(app) = weak.upgrade() {
                app.set_is_dragging_to_favorite(true);
                app.set_dragging_text(text.clone().into());
            }

            // ── 鼠标位置轮询：50ms 间隔更新坐标 ──
            // 鼠标离开窗口 → 立即启动 OLE 拖拽
            // query_continue_drag 中检测鼠标回到窗口 → 取消 OLE → 回到 Slint 拖拽
            let weak_poll = weak.clone();
            let hwnd = root_hwnd;

            fn poll_mouse_position(
                weak: slint::Weak<AppWindow>,
                hwnd: isize,
                text: String,
            ) {
                slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
                    if let Some(app) = weak.upgrade() {
                        // 停止轮询（拖拽已结束）
                        if !app.get_is_dragging_to_favorite() {
                            return;
                        }

                        if hwnd == 0 { return; }

                        #[cfg(target_os = "windows")]
                        {
                            #[link(name = "user32")]
                            extern "system" {
                                fn GetCursorPos(lpPoint: *mut i32) -> i32;
                                fn ClientToScreen(hWnd: isize, lpPoint: *mut i32) -> i32;
                                fn GetDpiForWindow(hWnd: isize) -> u32;
                                fn GetAsyncKeyState(vKey: i32) -> i16;
                                fn GetClientRect(hWnd: isize, lpRect: *mut i32) -> i32;
                            }

                            const VK_LBUTTON: i32 = 0x01;

                            unsafe {
                                let mut pt = [0i32; 2]; // x, y
                                if GetCursorPos(pt.as_mut_ptr()) != 0 {
                                    let mut origin = [0i32; 2];
                                    ClientToScreen(hwnd, origin.as_mut_ptr());

                                    let mouse_x_physical = pt[0] - origin[0];
                                    let mouse_y_physical = pt[1] - origin[1];

                                    let dpi = GetDpiForWindow(hwnd);
                                    let scale = dpi as f64 / 96.0;

                                    let mouse_x = (mouse_x_physical as f64 / scale) as f32;
                                    let mouse_y = (mouse_y_physical as f64 / scale) as f32;

                                    // 获取窗口客户区大小，判断鼠标是否在窗口内
                                    let mut rect = [0i32; 4];
                                    GetClientRect(hwnd, rect.as_mut_ptr());
                                    let client_w = (rect[2] as f64 / scale) as f32;
                                    let client_h = (rect[3] as f64 / scale) as f32;

                                    let is_outside = mouse_x < 0.0 || mouse_y < 0.0
                                        || mouse_x > client_w || mouse_y > client_h;

                                    if is_outside {
                                        // ── 鼠标离开窗口 → 立即启动 OLE 拖拽 ──
                                        eprintln!("[drag-out] Mouse outside, starting OLE drag");
                                        app.set_is_dragging_to_favorite(false);
                                        app.set_dragging_text(slint::SharedString::from(""));
                                        app.set_mouse_window_x(0.0);
                                        app.set_mouse_window_y(0.0);

                                        crate::drag_out::clear_reentry_flag();
                                        crate::drag_out::set_app_hwnd(hwnd);

                                        let text_ole = text.clone();
                                        let weak_ole = weak.clone();
                                        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                                            #[cfg(target_os = "windows")]
                                            {
                                                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                                                let ole_hwnd = unsafe { GetForegroundWindow() };
                                                crate::drag_out::start_drag_drop(&text_ole);

                                                // ── 检查是否因鼠标回到窗口而取消 OLE ──
                                                if crate::drag_out::was_reentry_cancelled() {
                                                    eprintln!("[drag-out] Re-entry detected, restarting Slint drag");
                                                    crate::drag_out::clear_reentry_flag();
                                                    if let Some(app) = weak_ole.upgrade() {
                                                        // 检查鼠标左键是否仍然按下
                                                        #[link(name = "user32")]
                                                        extern "system" {
                                                            fn GetAsyncKeyState(vKey: i32) -> i16;
                                                        }
                                                        const VK_LBUTTON: i32 = 0x01;
                                                        let still_pressed = unsafe { GetAsyncKeyState(VK_LBUTTON) < 0 };
                                                        if still_pressed {
                                                            app.set_is_dragging_to_favorite(true);
                                                            app.set_dragging_text(text_ole.clone().into());
                                                            poll_mouse_position(weak_ole.clone(), hwnd, text_ole.clone());
                                                        }
                                                    }
                                                } else {
                                                    // 正常拖放结束 → 重置鼠标状态
                                                    crate::drag_out::reset_mouse_state(ole_hwnd.0 as isize);
                                                }
                                            }
                                            eprintln!("[drag-out] OLE drag completed");
                                        });
                                        return;
                                    }

                                    // 更新 Slint 中的鼠标位置（供 toolbar 的 _is-over-favorite / _mouse-in-favorite 检测用）
                                    app.set_mouse_window_x(mouse_x.into());
                                    app.set_mouse_window_y(mouse_y.into());

                                    // ── 检测鼠标左键释放 ──
                                    let lbutton_down = GetAsyncKeyState(VK_LBUTTON) < 0;
                                    if !lbutton_down {
                                        eprintln!("[drag-out] Mouse button released, pos:({:.1},{:.1})", mouse_x, mouse_y);
                                        app.set_is_dragging_to_favorite(false);
                                        // 同时清除视觉状态（is-dragging）。
                                        // 如果鼠标在爱心上，toolbar 的 changed 处理器会触发
                                        // drag-over-favorite-button 重新设置状态。
                                        app.set_is_dragging(false);
                                        return;
                                    }
                                }
                            }

                            if app.get_is_dragging_to_favorite() {
                                poll_mouse_position(weak, hwnd, text);
                            }
                        }

                        #[cfg(not(target_os = "windows"))]
                        {
                            if app.get_is_dragging_to_favorite() {
                                poll_mouse_position(weak, hwnd, text);
                            }
                        }
                    }
                });
            }

            poll_mouse_position(weak_poll, hwnd, text.clone());
        }
    });
}

fn register_toggle_pin(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    app.on_toggle_pin(move || {
        if let Some(app) = weak.upgrade() {
            let new_pinned = !app.get_pinned();
            app.set_pinned(new_pinned);
            // 持久化到数据库
            let value = if new_pinned { "true" } else { "false" };
            state.set_config("pinned", value);
            eprintln!("[pin] Toggled pin state: {} (persisted)", new_pinned);
        }
    });
}

fn register_toggle_multi_select(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_toggle_multi_select(move |index: i32| {
        let idx = index as usize;
        if let Some(app) = weak.upgrade() {
            eprintln!("[multi-select] Entering multi-select mode, selected: {}", idx);
            app.set_multi_select_mode(true);

            // Build selected-states model: only the clicked item is selected
            let history = app.get_clipboard_history();
            let history_len = history.row_count();
            let mut states: Vec<bool> = vec![false; history_len];
            if idx < history_len {
                states[idx] = true;
            }
            let model = std::rc::Rc::new(slint::VecModel::from(states));
            let _ = app.set_selected_states(model.into());
            app.set_selected_count(1);
        }
    });
}

fn register_toggle_selection(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    app.on_toggle_selection(move |index: i32| {
        let idx = index as usize;
        if let Some(app) = weak.upgrade() {
            let current = app.get_selected_states();
            let count = current.row_count();
            let mut states: Vec<bool> = (0..count)
                .map(|i| current.row_data(i).unwrap_or(false))
                .collect();

            if idx < count {
                states[idx] = !states[idx];
            }

            let any_selected = states.iter().any(|&s| s);
            let selected_count = states.iter().filter(|&&s| s).count() as i32;
            eprintln!("[multi-select] Toggle {}, selected={}", idx, selected_count);

            if !any_selected {
                // No items selected → exit multi-select mode
                app.set_multi_select_mode(false);
                let model = std::rc::Rc::new(slint::VecModel::from(Vec::<bool>::new()));
                let _ = app.set_selected_states(model.into());
                app.set_selected_count(0);
            } else {
                let model = std::rc::Rc::new(slint::VecModel::from(states));
                let _ = app.set_selected_states(model.into());
                app.set_selected_count(selected_count);
            }
        }
    });
}

fn register_clear_multi_select(app: &AppWindow, _ctx: &CallbackContext) {
    let weak = _ctx.app_weak.clone();
    app.on_clear_multi_select(move || {
        if let Some(app) = weak.upgrade() {
            eprintln!("[multi-select] Clearing multi-select mode");
            app.set_multi_select_mode(false);
            let model = std::rc::Rc::new(slint::VecModel::from(Vec::<bool>::new()));
            let _ = app.set_selected_states(model.into());
            let text_model = std::rc::Rc::new(slint::VecModel::from(Vec::<slint::SharedString>::new()));
            let _ = app.set_multi_select_texts(text_model.into());
            app.set_selected_count(0);
        }
    });
}

fn register_search_history(app: &AppWindow, ctx: &CallbackContext) {
    let state = ctx.state.clone();
    let weak = ctx.app_weak.clone();
    let entries = ctx.clipboard_entries.clone();
    let app_data_dir = ctx.app_data_dir.clone();
    
    app.on_search_history(move |query: slint::SharedString| {
        let query_str = query.to_string();
        eprintln!("[search] Query: '{}'", query_str);
        
        crate::sync::sync_search_to_ui(&weak, &state, &entries, &app_data_dir, &query_str);
    });
}

fn register_reset_window_size(app: &AppWindow, ctx: &CallbackContext) {
    let weak = ctx.app_weak.clone();
    let state = ctx.state.clone();

    app.on_reset_window_size(move || {
        if let Some(app) = weak.upgrade() {
            // Default window size: 280x396
            app.window().set_size(slint::LogicalSize::new(280.0, 396.0));

            // Save to config
            let _ = state.set_config("window-width", "280");
            let _ = state.set_config("window-height", "396");

            eprintln!("[window] Reset to default size: 280x396");
        }
    });
}
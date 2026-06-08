use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use slint::ComponentHandle;
use crate::AppWindow;
use crate::ClipboardEntryData;
use crate::ContentKind;
use paste_bridge_core::models::ContentType;
use image::GenericImageView;

/// 防重入标记: 当 sync_history_to_ui_async 正在后台解码时,跳过后续的重复调用。
/// 避免在快速连续复制时产生大量并发解码线程导致 CPU 满载。
static SYNC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content_type: ContentType,
    pub text: slint::SharedString,
    pub image_path: Option<String>,  // 相对路径 "images/xxx.png",文本项 None
}

/// 后台解码好的图片数据,避免在 UI 线程调用 slint::Image::load_from_path
struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub fn truncate_for_display(text: &str, max_bytes: usize) -> slint::SharedString {
    if text.len() <= max_bytes {
        return text.into();
    }

    let mut cut_point = max_bytes;
    while cut_point > 0 && !text.is_char_boundary(cut_point) {
        cut_point -= 1;
    }

    let truncated = &text[..cut_point];

    if let Some(last_nl) = truncated.rfind('\n') {
        format!("{}\n...", &truncated[..last_nl]).into()
    } else {
        format!("{}...", truncated).into()
    }
}

pub fn sync_history_to_ui(
    weak: &slint::Weak<AppWindow>,
    state: &Arc<paste_bridge_core::state::AppState>,
    entries_lock: &Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    app_data_dir: &Path,
    trigger_animation: bool,
) {
    if let Some(w) = weak.upgrade() {
        let ascending = w.get_sort_ascending();
        let history = state.get_history(ascending);

        // === 内部 entries_lock: 同时含文本和图片,给 copy-item 用 id 查找 ===
        let mut internal_entries: Vec<ClipboardEntry> = Vec::with_capacity(history.len());
        for item in &history {
            let ct = item.content_type.clone();
            let text = item.content_text.clone().unwrap_or_default();
            let image_path = if matches!(ct, ContentType::Image) {
                item.content_path.clone()
            } else {
                None
            };
            internal_entries.push(ClipboardEntry {
                id: item.id,
                content_type: ct,
                text: text.into(),
                image_path,
            });
        }

        // === UI 列表数据: 文本 + 图片,统一为 ClipboardEntryData ===
        let ui_entries: Vec<ClipboardEntryData> = history.iter().map(|item| {
            let is_image = matches!(item.content_type, ContentType::Image);
            let kind = if is_image { ContentKind::Image } else { ContentKind::Text };

            let text = if is_image {
                String::new()
            } else {
                item.content_text.clone()
                    .map(|s| truncate_for_display(&s, 2000).to_string())
                    .unwrap_or_default()
            };

            // 图片: 加载到 slint::Image; 文本: 空 image (默认)
            let image = if is_image {
                item.content_path.as_ref()
                    .map(|rel| {
                        let abs = app_data_dir.join(rel);
                        match slint::Image::load_from_path(&abs) {
                            img @ Ok(_) => img.unwrap_or_default(),
                            Err(_) => {
                                eprintln!("[sync] skip corrupt PNG: {}", abs.display());
                                slint::Image::default()
                            }
                        }
                    })
                    .unwrap_or_default()
            } else {
                slint::Image::default()
            };

            ClipboardEntryData {
                id: item.id as i32,
                content_kind: kind,
                text: text.into(),
                image,
                timestamp: format_timestamp(item.created_at).into(),
                width: item.width.unwrap_or(0) as i32,
                height: item.height.unwrap_or(0) as i32,
                file_size: item.file_size.unwrap_or(0) as i32,
            }
        }).collect();

        // 旧 model: clipboard-history (文本) / clipboard-timestamps,供 favorite folder 视图与底部指示器
        let text_items: Vec<slint::SharedString> = internal_entries.iter()
            .map(|e| e.text.clone())
            .collect();
        let timestamps: Vec<slint::SharedString> = history.iter()
            .map(|item| format_timestamp(item.created_at).into())
            .collect();

        {
            let mut lock = entries_lock.lock().unwrap();
            *lock = internal_entries;
        }

        w.set_clipboard_history(std::rc::Rc::new(slint::VecModel::from(text_items)).into());
        w.set_clipboard_timestamps(std::rc::Rc::new(slint::VecModel::from(timestamps)).into());
        w.set_clipboard_entries(std::rc::Rc::new(slint::VecModel::from(ui_entries)).into());

        if trigger_animation {
            crate::animation::trigger_content_update_fade(w.as_weak());
        }
    }
}

/// 异步版本: 在后台线程解码图片,仅模型更新在 UI 线程完成。
///
/// 避免 `slint::Image::load_from_path` 在 UI 线程上同步解码多个 PNG
/// (尤其是大图如 3026×1806)导致的界面卡顿。
pub fn sync_history_to_ui_async(
    weak: slint::Weak<AppWindow>,
    state: Arc<paste_bridge_core::state::AppState>,
    entries_lock: Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    app_data_dir: std::path::PathBuf,
    trigger_animation: bool,
) {
    std::thread::spawn(move || {
        // 防重入: 如果上一次同步仍在解码中,跳过本次刷新以避免 CPU 满载
        if SYNC_IN_PROGRESS.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            eprintln!("[sync] sync still in progress, skipping redundant call");
            return;
        }

        // ── Phase 1: 后台线程 ── 查询 DB + 解码所有图片到 RGBA ──
        let (internal_entries, ui_data) = {
            // 读取排序方向(可以从 Slint handle 安全地读取属性)
            let ascending = weak.upgrade()
                .map(|w| w.get_sort_ascending())
                .unwrap_or(false);
            let history = state.get_history(ascending);

            let mut internal_entries = Vec::with_capacity(history.len());
            let mut ui_data: Vec<(
                i64,
                ContentType,
                String,                // display text
                slint::SharedString,   // full text for internal
                Option<String>,        // image path
                Option<DecodedImage>,
                String,                // formatted timestamp
                i32,                   // width
                i32,                   // height
                i32,                   // file_size
            )> = Vec::with_capacity(history.len());

            for item in &history {
                let is_image = matches!(item.content_type, ContentType::Image);
                let ct = item.content_type.clone();
                let text = item.content_text.clone().unwrap_or_default();

                // Internal entry
                let image_path = if is_image {
                    item.content_path.clone()
                } else {
                    None
                };
                internal_entries.push(ClipboardEntry {
                    id: item.id,
                    content_type: ct.clone(),
                    text: text.clone().into(),
                    image_path: image_path.clone(),
                });

                // UI entry text
                let text_display = if is_image {
                    String::new()
                } else {
                    item.content_text.clone()
                        .map(|s| truncate_for_display(&s, 2000).to_string())
                        .unwrap_or_default()
                };

                // 在后台线程解码 PNG → RGBA bytes (优先加载缩略图)
                let decoded = if is_image {
                    item.content_path.as_ref().and_then(|rel| {
                        // 缩略图路径: images/thumb_{hash}.png,不存在时回退到原图(向后兼容)
                        let thumb_rel = rel.replace("images/", "images/thumb_");
                        let thumb_abs = app_data_dir.join(&thumb_rel);
                        let abs = if thumb_abs.exists() {
                            thumb_abs
                        } else {
                            app_data_dir.join(rel)
                        };
                        match std::fs::read(&abs) {
                            Ok(bytes) => {
                                match image::load_from_memory(&bytes) {
                                    Ok(img) => {
                                        let (w, h) = img.dimensions();
                                        let rgba = img.to_rgba8().to_vec();
                                        Some(DecodedImage { width: w, height: h, rgba })
                                    }
                                    Err(e) => {
                                        eprintln!("[sync] skip corrupt PNG: {} ({})", abs.display(), e);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[sync] can't read PNG: {} ({})", abs.display(), e);
                                None
                            }
                        }
                    })
                } else {
                    None
                };

                ui_data.push((
                    item.id,
                    ct,
                    text_display,
                    text.into(),
                    image_path,
                    decoded,
                    format_timestamp(item.created_at),
                    item.width.unwrap_or(0) as i32,
                    item.height.unwrap_or(0) as i32,
                    item.file_size.unwrap_or(0) as i32,
                ));
            }

            (internal_entries, ui_data)
        };

        // ── Phase 2: UI 线程 ── 创建 slint::Image 并更新模型 ──
        let _ = slint::invoke_from_event_loop(move || {
            let w = match weak.upgrade() {
                Some(w) => w,
                None => {
                    // 窗口已销毁,释放防重入锁
                    SYNC_IN_PROGRESS.store(false, Ordering::SeqCst);
                    return;
                },
            };

            // 更新内部 entries (id → 内容映射,供 copy-item 使用)
            {
                let mut lock = entries_lock.lock().unwrap();
                *lock = internal_entries;
            }

            // text_items: 全文(给 clipboard-history 旧视图)
            let text_items: Vec<slint::SharedString> = ui_data.iter()
                .map(|(_, _, _, ref full, _, _, _, _, _, _)| full.clone())
                .collect();
            let timestamps: Vec<slint::SharedString> = ui_data.iter()
                .map(|(_, _, _, _, _, _, ref ts, _, _, _)| slint::SharedString::from(ts.as_str()))
                .collect();

            // 构建真正的 slint::Image (从预解码的 RGBA)
            let ui_entries: Vec<ClipboardEntryData> = ui_data.into_iter().map(|(id, ct, text_display, _, _, decoded, timestamp, width, height, file_size)| {
                let is_image = matches!(ct, ContentType::Image);
                let kind = if is_image { ContentKind::Image } else { ContentKind::Text };

                let image = match decoded {
                    Some(DecodedImage { width: w, height: h, rgba }) => {
                        let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&rgba, w, h);
                        slint::Image::from_rgba8(buffer)
                    }
                    None => slint::Image::default(),
                };

                ClipboardEntryData {
                    id: id as i32,
                    content_kind: kind,
                    text: text_display.into(),
                    image,
                    timestamp: timestamp.into(),
                    width,
                    height,
                    file_size,
                }
            }).collect();

            w.set_clipboard_history(std::rc::Rc::new(slint::VecModel::from(text_items)).into());
            w.set_clipboard_timestamps(std::rc::Rc::new(slint::VecModel::from(timestamps)).into());
            w.set_clipboard_entries(std::rc::Rc::new(slint::VecModel::from(ui_entries)).into());

            if trigger_animation {
                crate::animation::trigger_content_update_fade(w.as_weak());
            }

            // 在 UI 更新真正完成后才释放防重入锁
            SYNC_IN_PROGRESS.store(false, Ordering::SeqCst);
        });
    });
}

/// 搜索历史记录并同步到 UI
pub fn sync_search_to_ui(
    weak: &slint::Weak<AppWindow>,
    state: &Arc<paste_bridge_core::state::AppState>,
    entries_lock: &Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    app_data_dir: &Path,
    query: &str,
) {
    if let Some(w) = weak.upgrade() {
        let ascending = w.get_sort_ascending();
        
        // 如果查询为空，显示全部历史
        let history = if query.is_empty() {
            state.get_history(ascending)
        } else {
            state.search_history(query, ascending)
        };

        // 更新 has_search_results 属性
        w.set_has_search_results(!history.is_empty());

        // === 内部 entries_lock ===
        let mut internal_entries: Vec<ClipboardEntry> = Vec::with_capacity(history.len());
        for item in &history {
            let ct = item.content_type.clone();
            let text = item.content_text.clone().unwrap_or_default();
            let image_path = if matches!(ct, ContentType::Image) {
                item.content_path.clone()
            } else {
                None
            };
            internal_entries.push(ClipboardEntry {
                id: item.id,
                content_type: ct,
                text: text.into(),
                image_path,
            });
        }

        // === UI 列表数据 ===
        let ui_entries: Vec<ClipboardEntryData> = history.iter().map(|item| {
            let is_image = matches!(item.content_type, ContentType::Image);
            let kind = if is_image { ContentKind::Image } else { ContentKind::Text };

            let text = if is_image {
                String::new()
            } else {
                item.content_text.clone()
                    .map(|s| truncate_for_display(&s, 2000).to_string())
                    .unwrap_or_default()
            };

            let image = if is_image {
                item.content_path.as_ref()
                    .map(|rel| {
                        let abs = app_data_dir.join(rel);
                        match slint::Image::load_from_path(&abs) {
                            img @ Ok(_) => img.unwrap_or_default(),
                            Err(_) => {
                                eprintln!("[search] skip corrupt PNG: {}", abs.display());
                                slint::Image::default()
                            }
                        }
                    })
                    .unwrap_or_default()
            } else {
                slint::Image::default()
            };

            ClipboardEntryData {
                id: item.id as i32,
                content_kind: kind,
                text: text.into(),
                image,
                timestamp: format_timestamp(item.created_at).into(),
                width: item.width.unwrap_or(0) as i32,
                height: item.height.unwrap_or(0) as i32,
                file_size: item.file_size.unwrap_or(0) as i32,
            }
        }).collect();

        // 更新内部 entries
        {
            let mut lock = entries_lock.lock().unwrap();
            *lock = internal_entries;
        }

        // 更新 UI 模型
        let text_items: Vec<slint::SharedString> = history.iter()
            .map(|item| item.content_text.clone().unwrap_or_default().into())
            .collect();
        let timestamps: Vec<slint::SharedString> = history.iter()
            .map(|item| format_timestamp(item.created_at).into())
            .collect();

        w.set_clipboard_history(std::rc::Rc::new(slint::VecModel::from(text_items)).into());
        w.set_clipboard_timestamps(std::rc::Rc::new(slint::VecModel::from(timestamps)).into());
        w.set_clipboard_entries(std::rc::Rc::new(slint::VecModel::from(ui_entries)).into());
    }
}

/// Format a relative time string from a Unix timestamp in **milliseconds**.
///
/// Returns human-readable relative times like "刚刚", "5分钟前", "2小时前", "3天前".
fn format_timestamp(created_at_ms: i64) -> String {
    use chrono::{DateTime, Local, TimeZone};
    let Some(dt) = Local.timestamp_millis_opt(created_at_ms).single() else {
        return String::new();
    };
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

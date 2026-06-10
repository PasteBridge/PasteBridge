use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use std::collections::HashMap;
use std::sync::Mutex;
use slint::ComponentHandle;
use crate::AppWindow;
use crate::ClipboardEntryData;
use crate::ContentKind;
use paste_bridge_core::models::ContentType;

/// 防重入标记: 当 sync_history_to_ui_async 正在后台解码时,跳过后续的重复调用。
/// 避免在快速连续复制时产生大量并发解码线程导致 CPU 满载。
static SYNC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// 时间戳格式化缓存:
/// key   = (id, created_at_ms, now_minute_bucket)
/// value = 格式化好的相对时间字符串
/// 相对时间在分钟粒度上是稳定的;通过缓存避免每次搜索重建 ListView 时
/// 对相同条目反复执行 format_timestamp,降低搜索输入时的帧率抖动。
static TIMESTAMP_CACHE: Mutex<Option<HashMap<(i64, i64, i64), String>>> = Mutex::new(None);

fn format_timestamp_cached(id: i64, created_at_ms: i64) -> String {
    use chrono::{DateTime, Local, TimeZone};
    let now: DateTime<Local> = Local::now();
    let minute_bucket = now.timestamp() / 60;
    let key = (id, created_at_ms, minute_bucket);

    let mut guard = TIMESTAMP_CACHE.lock().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(v) = cache.get(&key) {
        return v.clone();
    }
    let v = format_timestamp_inner(created_at_ms, &now);
    cache.insert(key, v.clone());
    v
}

#[derive(Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content_type: ContentType,
    pub text: slint::SharedString,
    /// 图片项: 数据库 id (用于懒加载从 DB 读取 BLOB); 文本项: 0
    pub image_id: i64,
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
            let image_id = if matches!(ct, ContentType::Image) {
                item.id
            } else {
                0
            };
            internal_entries.push(ClipboardEntry {
                id: item.id,
                content_type: ct,
                text: text.into(),
                image_id,
            });
        }

        // === UI 列表数据: 文本 + 图片,统一为 ClipboardEntryData ===
        // 图片按需懒加载: 此处只填充 image-id,实际解码由 load_visible_images 回调处理
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

            ClipboardEntryData {
                id: item.id as i32,
                content_kind: kind,
                text: text.into(),
                image: slint::Image::default(),
                image_loaded: !is_image, // 文本标记为已加载,图片标记为未加载
                image_id: if is_image { item.id as i32 } else { 0 },
                timestamp: format_timestamp(item.id, item.created_at).into(),
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
            .map(|item| format_timestamp(item.id, item.created_at).into())
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

        // 触发首屏可见区域的图片懒加载
        let window_height = w.window().size().to_logical(w.window().scale_factor()).height;
        let visible_count = (window_height / 144.0).ceil() as i32 + 2;
        w.invoke_load_visible_images(0, visible_count.max(10));

        // app_data_dir 不再被使用,保留参数以兼容调用方签名
        let _ = app_data_dir;
    }
}

/// 异步版本: 在后台线程解码图片,仅模型更新在 UI 线程完成。
///
/// 优化: 第一阶段快速加载所有元数据(文本+图片id),第二阶段按需解码图片。
/// 避免初始加载时解码所有图片导致的卡顿。
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

        // ── Phase 1: 快速加载元数据 ──
        // 只加载文本和图片id,不解码图片,实现快速首屏渲染
        let (internal_entries, ui_entries_metadata) = {
            let ascending = weak.upgrade()
                .map(|w| w.get_sort_ascending())
                .unwrap_or(false);
            let history = state.get_history(ascending);

            let mut internal_entries = Vec::with_capacity(history.len());
            let mut ui_entries_metadata = Vec::with_capacity(history.len());

            for item in &history {
                let is_image = matches!(item.content_type, ContentType::Image);
                let ct = item.content_type.clone();
                let text = item.content_text.clone().unwrap_or_default();

                // Internal entry
                let image_id = if is_image { item.id } else { 0 };
                internal_entries.push(ClipboardEntry {
                    id: item.id,
                    content_type: ct.clone(),
                    text: text.clone().into(),
                    image_id,
                });

                // UI entry metadata (图片未加载)
                let text_display = if is_image {
                    String::new()
                } else {
                    item.content_text.clone()
                        .map(|s| truncate_for_display(&s, 2000).to_string())
                        .unwrap_or_default()
                };

                ui_entries_metadata.push((
                    item.id,
                    ct,
                    text_display,
                    slint::SharedString::from(text.as_str()),
                    format_timestamp(item.id, item.created_at),
                    item.width.unwrap_or(0) as i32,
                    item.height.unwrap_or(0) as i32,
                    item.file_size.unwrap_or(0) as i32,
                ));
            }

            (internal_entries, ui_entries_metadata)
        };

        // ── Phase 2: UI 线程更新 ──
        // 创建空的 ClipboardEntryData,图片标记为未加载
        let _ = slint::invoke_from_event_loop(move || {
            let w = match weak.upgrade() {
                Some(w) => w,
                None => {
                    SYNC_IN_PROGRESS.store(false, Ordering::SeqCst);
                    return;
                },
            };

            // 更新内部 entries
            {
                let mut lock = entries_lock.lock().unwrap();
                *lock = internal_entries;
            }

            // 构建 UI entries (图片未加载)
            let text_items: Vec<slint::SharedString> = ui_entries_metadata.iter()
                .map(|(_, _, _, ref full, _, _, _, _)| full.clone())
                .collect();
            let timestamps: Vec<slint::SharedString> = ui_entries_metadata.iter()
                .map(|(_, _, _, _, ref ts, _, _, _)| slint::SharedString::from(ts.as_str()))
                .collect();

            let ui_entries: Vec<ClipboardEntryData> = ui_entries_metadata.into_iter()
                .map(|(id, ct, text_display, _, timestamp, width, height, file_size)| {
                    let is_image = matches!(ct, ContentType::Image);
                    let kind = if is_image { ContentKind::Image } else { ContentKind::Text };

                    ClipboardEntryData {
                        id: id as i32,
                        content_kind: kind,
                        text: text_display.into(),
                        image: slint::Image::default(), // 空图片,按需加载
                        image_loaded: false,            // 标记为未加载
                        image_id: if is_image { id as i32 } else { 0 },
                        timestamp: timestamp.into(),
                        width,
                        height,
                        file_size,
                    }
                })
                .collect();

            w.set_clipboard_history(std::rc::Rc::new(slint::VecModel::from(text_items)).into());
            w.set_clipboard_timestamps(std::rc::Rc::new(slint::VecModel::from(timestamps)).into());
            w.set_clipboard_entries(std::rc::Rc::new(slint::VecModel::from(ui_entries)).into());

            // 模型更新后立即触发可见区域图片加载,否则图片不会自动显示(需要滚动才会触发)
            let window_height = w.window().size().to_logical(w.window().scale_factor()).height;
            let visible_count = (window_height / 144.0).ceil() as i32 + 2;
            w.invoke_load_visible_images(0, visible_count.max(10));

            if trigger_animation {
                crate::animation::trigger_content_update_fade(w.as_weak());
            }

            SYNC_IN_PROGRESS.store(false, Ordering::SeqCst);
        });

        let _ = app_data_dir;
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
            let image_id = if matches!(ct, ContentType::Image) {
                item.id
            } else {
                0
            };
            internal_entries.push(ClipboardEntry {
                id: item.id,
                content_type: ct,
                text: text.into(),
                image_id,
            });
        }

        // === UI 列表数据: 按需懒加载图片 ===
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

            ClipboardEntryData {
                id: item.id as i32,
                content_kind: kind,
                text: text.into(),
                image: slint::Image::default(),
                image_loaded: !is_image, // 文本标记为已加载,图片标记为未加载
                image_id: if is_image { item.id as i32 } else { 0 },
                timestamp: format_timestamp(item.id, item.created_at).into(),
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
            .map(|item| format_timestamp(item.id, item.created_at).into())
            .collect();

        w.set_clipboard_history(std::rc::Rc::new(slint::VecModel::from(text_items)).into());
        w.set_clipboard_timestamps(std::rc::Rc::new(slint::VecModel::from(timestamps)).into());
        w.set_clipboard_entries(std::rc::Rc::new(slint::VecModel::from(ui_entries)).into());

        // 触发首屏图片懒加载
        let window_height = w.window().size().to_logical(w.window().scale_factor()).height;
        let visible_count = (window_height / 144.0).ceil() as i32 + 2;
        w.invoke_load_visible_images(0, visible_count.max(10));

        let _ = app_data_dir;
    }
}

/// Format a relative time string from a Unix timestamp in **milliseconds**.
///
/// Returns human-readable relative times like "刚刚", "5分钟前", "2小时前", "3天前".
fn format_timestamp_inner(created_at_ms: i64, now: &chrono::DateTime<chrono::Local>) -> String {
    use chrono::TimeZone;
    let Some(dt) = chrono::Local.timestamp_millis_opt(created_at_ms).single() else {
        return String::new();
    };
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

/// 模块内时间戳格式化入口:走缓存,避免搜索重建 ListView 时重复格式化
fn format_timestamp(id: i64, created_at_ms: i64) -> String {
    format_timestamp_cached(id, created_at_ms)
}


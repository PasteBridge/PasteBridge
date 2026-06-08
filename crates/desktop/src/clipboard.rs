use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use arboard::Clipboard;

/// Windows: 轻量级剪贴板序列号检测,跳过未变更时的全量图片读取。
#[cfg(target_os = "windows")]
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

/// 上次轮询时的剪贴板序列号(Windows Only)。
/// 序列号不变 = 剪贴板内容无变化,可跳过全部读写操作。
#[cfg(target_os = "windows")]
static LAST_CLIPBOARD_SEQ: AtomicU64 = AtomicU64::new(u64::MAX);

/// 全局剪贴板访问互斥锁。
///
/// Win32 的 OpenClipboard 一次只允许一个线程持有锁。剪贴板监听线程
/// (每 150ms 轮询) 和复制线程必须串行访问,否则会出现 os error 1418
/// ("线程没有打开的剪贴板")。
pub static CLIPBOARD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 跳过剪贴板监听线程的下一次图片检测。
/// 
/// 当应用主动向剪贴板写入图片后设置此标志,监听线程将跳过对"刚写入的"图片的
/// 重复检测和编码,避免不必要的 CPU 消耗和大图 PNG 编码导致的界面卡顿。
pub static SKIP_NEXT_IMAGE_DETECT: AtomicBool = AtomicBool::new(false);

/// 标记监听线程应跳过下一次图片检测。
pub fn skip_next_image_detect() {
    SKIP_NEXT_IMAGE_DETECT.store(true, Ordering::SeqCst);
}

/// 跳过剪贴板监听线程的下一次文本检测。
///
/// 当应用主动向剪贴板写入文本后设置此标志,监听线程将跳过对"刚写入的"文本的
/// 重复检测和全量 UI 刷新(包含图片重加载),避免界面卡顿。
pub static SKIP_NEXT_TEXT_DETECT: AtomicBool = AtomicBool::new(false);

/// 标记监听线程应跳过下一次文本检测。
pub fn skip_next_text_detect() {
    SKIP_NEXT_TEXT_DETECT.store(true, Ordering::SeqCst);
}

/// set_clipboard_image_blocking 成功写入剪贴板后,记录写入的 RGBA 数据的 hash 和长度。
/// 监听线程的 skip 分支直接使用这些值更新去重状态,避免重新读取剪贴板字节时
/// 因 arboard 在不同读操作间返回不同原始字节(颜色格式/alpha 处理差异)而导致
/// 同一个图片被反复检测为"新内容"并重复执行 PNG 编码+缩略图生成。
static EXPECTED_IMAGE_HASH: AtomicU64 = AtomicU64::new(0);
static EXPECTED_IMAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

/// 设置剪贴板文本。同步等待写入完成。
///
/// # Returns
/// * `Ok(())` - 成功
/// * `Err(String)` - 失败信息
pub fn set_clipboard_text_blocking(text: String) -> Result<(), String> {
    let _guard = CLIPBOARD_LOCK.lock().unwrap();

    let mut last_err = String::new();
    for attempt in 0..5 {
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text.clone()) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err = format!("set_text: {}", e);
                    eprintln!(
                        "set_clipboard_text_blocking attempt {} failed: {}",
                        attempt + 1,
                        last_err
                    );
                    thread::sleep(Duration::from_millis(20 * (attempt + 1) as u64));
                }
            },
            Err(e) => {
                last_err = format!("Clipboard::new: {}", e);
                eprintln!(
                    "set_clipboard_text_blocking attempt {} open failed: {}",
                    attempt + 1,
                    last_err
                );
                thread::sleep(Duration::from_millis(20 * (attempt + 1) as u64));
            }
        }
    }
    Err(last_err)
}

/// 异步版本(向后兼容):在新线程中调用同步版本。
pub fn set_clipboard_text_async(text: String) {
    thread::spawn(move || {
        if let Err(e) = set_clipboard_text_blocking(text) {
            eprintln!("set_clipboard_text_async 最终失败: {}", e);
        }
    });
}

pub fn start_clipboard_monitor<F>(state: Arc<paste_bridge_core::state::AppState>, on_change: F, app_data_dir: std::path::PathBuf)
where
    F: Fn() + Send + 'static,
{
    thread::spawn(move || {
        eprintln!("[core:clipboard] Monitoring thread started");

        // 去重状态(本线程本地)
        let mut last_text_hash: u64 = 0;
        let mut last_text_len: usize = 0;
        let mut last_image_hash: u64 = 0;
        let mut last_image_size: usize = 0;

        // 初始静默读取,避免启动时把当前剪贴板内容当成新拷贝
        {
            let _guard = CLIPBOARD_LOCK.lock().unwrap();
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    if let Ok(content) = clipboard.get_text() {
                        last_text_hash = content_hash(content.as_bytes());
                        last_text_len = content.len();
                    }
                    if let Ok(img) = clipboard.get_image() {
                        last_image_hash = content_hash(&img.bytes);
                        last_image_size = img.bytes.len();
                    }
                }
                Err(e) => {
                    eprintln!("[core:clipboard] Failed to create clipboard at init: {}", e);
                    return;
                }
            }
        }
        eprintln!("[core:clipboard] Clipboard initialized");

        loop {
            thread::sleep(Duration::from_millis(150));

            // ── Windows: 轻量级序列号检测 ──
            // GetClipboardSequenceNumber 仅读取一个全局计数器,远快于 get_image()
            // 序列号未变则剪贴板内容无变化,跳过全部读写操作。
            #[cfg(target_os = "windows")]
            {
                let seq = unsafe { GetClipboardSequenceNumber() } as u64;
                let prev = LAST_CLIPBOARD_SEQ.load(Ordering::Relaxed);
                if seq == prev {
                    continue;
                }
                LAST_CLIPBOARD_SEQ.store(seq, Ordering::Relaxed);
            }

            let mut need_on_change = false;
            // 从锁内提取的原始图片数据(RGBA),在锁外做编码和缩略图
            let mut pending_image: Option<(Vec<u8>, usize, usize)> = None; // (raw_rgba, width, height)
            let mut pending_text: Option<String> = None;

            // ══════ 锁范围: 仅做剪贴板读写 ══════
            // 不要把 PNG 编码/缩略图生成放在这里,否则 copy-item 会被阻塞数秒
            {
                let _guard = CLIPBOARD_LOCK.lock().unwrap();
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        // === 图片监听 ===
                        if SKIP_NEXT_IMAGE_DETECT.swap(false, Ordering::SeqCst) {
                            // 使用 set_clipboard_image_blocking 记录的预期 hash/size,
                            // 避免重读剪贴板字节因格式差异导致 hash 不同而重复检测
                            last_image_hash = EXPECTED_IMAGE_HASH.swap(0, Ordering::SeqCst);
                            last_image_size = EXPECTED_IMAGE_SIZE.swap(0, Ordering::SeqCst);
                            // 同步刷新文本去重状态(仍在锁内,安全)
                            if let Ok(content) = clipboard.get_text() {
                                last_text_hash = content_hash(content.as_bytes());
                                last_text_len = content.len();
                            }
                            eprintln!(
                                "[core:clipboard] Skip image detect: hash={} size={}",
                                last_image_hash, last_image_size
                            );
                        } else {
                            match clipboard.get_image() {
                                Ok(img) => {
                                    let size = img.bytes.len();
                                    let h = content_hash(&img.bytes);
                                    if size != last_image_size || h != last_image_hash {
                                        last_image_size = size;
                                        last_image_hash = h;
                                        // 只提取原始 RGBA 数据,编码到锁外再做
                                        pending_image = Some((img.bytes.to_vec(), img.width, img.height));

                                        // 刷新文本去重状态(还在锁内,安全)
                                        if let Ok(content) = clipboard.get_text() {
                                            last_text_hash = content_hash(content.as_bytes());
                                            last_text_len = content.len();
                                        }
                                    }
                                }
                                Err(_) => {
                                    last_image_hash = 0;
                                    last_image_size = 0;
                                }
                            }
                        }

                        // === 文本监听 ===
                        if SKIP_NEXT_TEXT_DETECT.swap(false, Ordering::SeqCst) {
                            if let Ok(content) = clipboard.get_text() {
                                last_text_hash = content_hash(content.as_bytes());
                                last_text_len = content.len();
                                eprintln!(
                                    "[core:clipboard] Skip text detect: {} bytes",
                                    last_text_len
                                );
                            }
                        } else if let Ok(content) = clipboard.get_text() {
                            if !content.is_empty() {
                                let current_len = content.len();
                                let current_hash = content_hash(content.as_bytes());
                                if current_len != last_text_len || current_hash != last_text_hash {
                                    last_text_len = current_len;
                                    last_text_hash = current_hash;
                                    pending_text = Some(content.clone());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[core:clipboard] Open clipboard failed: {}", e);
                    }
                }
            } // ══════ CLIPBOARD_LOCK 已释放 ══════

            // ══════ 锁外处理: PNG 编码 + 缩略图(CPU 密集) ══════
            if let Some((raw_rgba, width, height)) = pending_image {
                match encode_rgba_to_png(width as u32, height as u32, &raw_rgba) {
                    Ok(png_bytes) => {
                        eprintln!(
                            "[core:clipboard] New image: {}x{}, {} bytes PNG (raw RGBA {} bytes)",
                            width, height, png_bytes.len(), raw_rgba.len()
                        );
                        match state.push_image(
                            &png_bytes,
                            "image/png",
                            width as i32,
                            height as i32,
                        ) {
                            Some((id, path)) => {
                                eprintln!(
                                    "[core:clipboard] Image stored id={} path={}",
                                    id, path
                                );

                                // 生成缩略图(最大 200px 宽,保持宽高比)
                                let thumb_size = 200u32;
                                let (thumb_w, thumb_h) = if width > height {
                                    (thumb_size, (thumb_size as f64 * height as f64 / width as f64) as u32)
                                } else {
                                    ((thumb_size as f64 * width as f64 / height as f64) as u32, thumb_size)
                                };
                                if let Some(thumb_img) = image::RgbaImage::from_raw(width as u32, height as u32, raw_rgba) {
                                    let thumb_resized = image::imageops::resize(
                                        &thumb_img,
                                        thumb_w.max(1),
                                        thumb_h.max(1),
                                        image::imageops::FilterType::Lanczos3,
                                    );
                                    if let Ok(thumb_png) = encode_rgba_to_png(
                                        thumb_resized.width(),
                                        thumb_resized.height(),
                                        thumb_resized.as_raw(),
                                    ) {
                                        let thumb_path = path.replace("images/", "images/thumb_");
                                        let thumb_abs = app_data_dir.join(&thumb_path);
                                        if let Some(parent) = thumb_abs.parent() {
                                            let _ = std::fs::create_dir_all(parent);
                                        }
                                        let _ = std::fs::write(&thumb_abs, &thumb_png);
                                    }
                                }

                                need_on_change = true;
                            }
                            None => {
                                eprintln!("[core:clipboard] Failed to store image (DB error)");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[core:clipboard] PNG encode failed: {}", e);
                    }
                }
            }

            // ══════ 锁外处理: 文本存储 ══════
            if let Some(text) = pending_text {
                state.push_clipboard(text.clone());
                eprintln!(
                    "[core:clipboard] New text detected: {}",
                    text.chars().take(50).collect::<String>()
                );
                need_on_change = true;
            }

            if need_on_change {
                on_change();
            }
        }
    });
}

fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn encode_rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgba, ImageFormat};
    let buf: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| format!("Invalid image buffer: {}x{} with {} bytes", width, height, rgba.len()))?;

    let mut out = Vec::new();
    {
        let mut cursor = Cursor::new(&mut out);
        buf.write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("PNG encode failed: {}", e))?;
    }
    Ok(out)
}

/// 同步读取 PNG 文件并写入系统剪贴板(图片)。
///
/// 必须同步等待完成,因为调用方需要在图片确实在剪贴板上之后
/// 才向目标窗口发送 Ctrl+V。
///
/// # Returns
/// * `Ok((w, h))` - 成功,返回图片尺寸
/// * `Err(String)` - 失败信息
pub fn set_clipboard_image_blocking(png_path: &std::path::Path) -> Result<(u32, u32), String> {
    // 读取 + 解码 PNG
    let png_data = std::fs::read(png_path)
        .map_err(|e| format!("读取文件失败 {}: {}", png_path.display(), e))?;

    let img = image::load_from_memory(&png_data)
        .map_err(|e| format!("PNG 解码失败 {}: {}", png_path.display(), e))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let raw: Vec<u8> = img.into_raw();

    // 持有全局锁,串行访问剪贴板
    let _guard = CLIPBOARD_LOCK.lock().unwrap();

    let mut last_err = String::new();
    for attempt in 0..8 {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                let img_data = arboard::ImageData {
                    width: w as usize,
                    height: h as usize,
                    bytes: raw.clone().into(),
                };
                match clipboard.set_image(img_data) {
                    Ok(_) => {
                        eprintln!(
                            "set_clipboard_image: 写入 {}x{} 图片自 {}",
                            w,
                            h,
                            png_path.display()
                        );
                        // 记录此次写入的 RGBA 数据的 hash 和大小,
                        // 供监听线程的 skip 分支直接使用,无需重新读取剪贴板
                        EXPECTED_IMAGE_HASH.store(content_hash(&raw), Ordering::SeqCst);
                        EXPECTED_IMAGE_SIZE.store(raw.len(), Ordering::SeqCst);
                        return Ok((w, h));
                    }
                    Err(e) => {
                        last_err = format!("set_image: {}", e);
                        eprintln!(
                            "set_clipboard_image attempt {} 失败: {}",
                            attempt + 1,
                            last_err
                        );
                        thread::sleep(Duration::from_millis(15 * (attempt + 1) as u64));
                    }
                }
            }
            Err(e) => {
                last_err = format!("打开剪贴板: {}", e);
                eprintln!(
                    "set_clipboard_image attempt {} 打开失败: {}",
                    attempt + 1,
                    last_err
                );
                thread::sleep(Duration::from_millis(15 * (attempt + 1) as u64));
            }
        }
    }
    Err(last_err)
}

/// 异步版本(向后兼容):在新线程中调用同步版本。
/// 内部已经处理好锁和重试。
pub fn set_clipboard_image(png_path: &std::path::Path) {
    let png_path = png_path.to_path_buf();
    thread::spawn(move || {
        if let Err(e) = set_clipboard_image_blocking(&png_path) {
            eprintln!("set_clipboard_image 最终失败 ({}): {}", png_path.display(), e);
        }
    });
}

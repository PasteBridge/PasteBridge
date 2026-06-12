use std::thread;

pub fn set_clipboard_text(text: String) {
    // arboard 仅在桌面端 (Windows/macOS/Linux with X11/Wayland) 可用。
    // Android 端走 UniFFI + JNI 直接调系统 ClipboardManager,不需要这个 stub。
    // 在 Android target 下此函数体为空,编译期直接抹掉 arboard 引用。
    #[cfg(not(target_os = "android"))]
    {
        thread::spawn(move || {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text);
            }
        });
    }
    // Android 下 text 故意被 drop;上层 (Kotlin SyncService.onRemoteCopy) 会自己写系统剪贴板。
    #[cfg(target_os = "android")]
    {
        let _ = text;
    }
}
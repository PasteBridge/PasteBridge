use std::thread;
use std::time::Duration;

/// 将文本写入系统剪贴板（text 必须是_owned 的，即 clone 或 String）
pub fn set_clipboard_text(text: String) {
    thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    });
}

pub fn start_clipboard_monitor<F>(state: std::sync::Arc<crate::core::state::AppState>, on_new_text: F)
where
    F: Fn(String) + Send + 'static,
{
    thread::spawn(move || {
        eprintln!("Clipboard monitoring thread started");
        use arboard::Clipboard;
        
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create clipboard: {}", e);
                return;
            }
        };
        
        let mut last_content = String::new();
        if let Ok(content) = clipboard.get_text() {
            last_content = content;
        }
        eprintln!("Clipboard created successfully");

        loop {
            thread::sleep(Duration::from_millis(500));

            if let Ok(content) = clipboard.get_text() {
                if !content.is_empty() && content != last_content {
                    last_content = content.clone();
                    let preview: String = content.chars().take(50).collect();
                    eprintln!("New clipboard content: {}", preview);

                    // Push to state (handles deduplication)
                    state.push_clipboard(content.clone());
                    // Notify via callback
                    on_new_text(content);
                }
            }
        }
    });
}

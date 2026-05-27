use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use arboard::Clipboard;

pub fn start_clipboard_monitor<F>(state: Arc<paste_bridge_core::state::AppState>, on_change: F)
where
    F: Fn() + Send + 'static,
{
    thread::spawn(move || {
        eprintln!("[core:clipboard] Monitoring thread started");

        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[core:clipboard] Failed to create clipboard: {}", e);
                return;
            }
        };

        let mut last_hash: u64 = 0;
        let mut last_len: usize = 0;
        if let Ok(content) = clipboard.get_text() {
            last_hash = content_hash(&content);
            last_len = content.len();
        }
        eprintln!("[core:clipboard] Clipboard initialized");

        loop {
            thread::sleep(Duration::from_millis(800));

            if let Ok(content) = clipboard.get_text() {
                if !content.is_empty() {
                    let current_len = content.len();
                    let current_hash = content_hash(&content);
                    if current_len == last_len && current_hash == last_hash {
                        continue;
                    }
                    last_len = current_len;
                    last_hash = current_hash;

                    state.push_clipboard(content.clone());

                    eprintln!("[core:clipboard] New content detected: {}",
                        content.chars().take(50).collect::<String>());

                    on_change();
                }
            }
        }
    });
}

fn content_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
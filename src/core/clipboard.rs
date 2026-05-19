//! Clipboard monitoring and management

use std::thread;
use std::time::Duration;
use std::sync::Arc;
use arboard::Clipboard;

/// Event sent when clipboard content changes
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub text: String,
}

/// Start clipboard monitoring
/// Calls `on_change` callback when clipboard content changes
pub fn start_clipboard_monitor<F>(state: Arc<crate::core::state::AppState>, on_change: F)
where
    F: Fn(String) + Send + 'static,
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
        
        // Initialize with current clipboard content
        let mut last_content = String::new();
        if let Ok(content) = clipboard.get_text() {
            last_content = content;
        }
        eprintln!("[core:clipboard] Clipboard initialized");

        loop {
            thread::sleep(Duration::from_millis(500));

            if let Ok(content) = clipboard.get_text() {
                if !content.is_empty() && content != last_content {
                    last_content = content.clone();
                    
                    // Update state
                    state.push_clipboard(content.clone());
                    
                    eprintln!("[core:clipboard] New content detected: {}", 
                        content.chars().take(50).collect::<String>());
                    
                    // Notify callback
                    on_change(content);
                }
            }
        }
    });
}

/// Set text to system clipboard
pub fn set_clipboard_text(text: String) {
    thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    });
}

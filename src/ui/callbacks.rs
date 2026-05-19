//! UI callbacks - bridge between Slint UI and backend

use crate::core::clipboard as core_clipboard;

/// Handle copy item action
pub fn on_copy_item(text: String) {
    eprintln!("[ui:callback] copy_item called: {}", 
        text.chars().take(20).collect::<String>());
    
    // Set to system clipboard
    core_clipboard::set_clipboard_text(text);
}

/// Handle hide window action
pub fn on_hide_window() {
    eprintln!("[ui:callback] hide_window called");
}

/// Handle minimize window action
pub fn on_minimize_window() {
    eprintln!("[ui:callback] minimize_window called");
}

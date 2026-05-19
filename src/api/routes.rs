//! API routes handling

use std::sync::Arc;
use crate::core::state::AppState;
use crate::core::clipboard as core_clipboard;

/// Handle GET /clipboard/history
pub fn handle_get_history(state: &Arc<AppState>) -> Vec<u8> {
    let history = state.get_history();
    serde_json::to_vec(&history).unwrap_or_default()
}

/// Handle POST /clipboard/copy
pub fn handle_copy(state: &Arc<AppState>, body: &[u8]) -> Result<(), String> {
    let text = String::from_utf8_lossy(body).to_string();
    
    // Add to history
    state.push_clipboard(text.clone());
    
    // Set to system clipboard
    core_clipboard::set_clipboard_text(text);
    
    Ok(())
}

/// Handle POST /window/show
pub fn handle_window_show(state: &Arc<AppState>) -> Result<(), String> {
    state.set_window_visible(true);
    Ok(())
}

/// Handle POST /window/hide
pub fn handle_window_hide(state: &Arc<AppState>) -> Result<(), String> {
    state.set_window_visible(false);
    Ok(())
}

/// Handle GET /window/visible
pub fn handle_get_visible(state: &Arc<AppState>) -> Vec<u8> {
    let visible = state.is_window_visible();
    serde_json::to_vec(&visible).unwrap_or_default()
}

/// Handle POST /clipboard/clear
pub fn handle_clear(state: &Arc<AppState>) -> Result<(), String> {
    state.clear_history();
    Ok(())
}

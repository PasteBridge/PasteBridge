use std::sync::Arc;
use crate::state::AppState;
use crate::clipboard as core_clipboard;

pub fn handle_get_history(state: &Arc<AppState>) -> Vec<u8> {
    let history = state.get_history();
    serde_json::to_vec(&history).unwrap_or_default()
}

pub fn handle_copy(state: &Arc<AppState>, body: &[u8]) -> Result<(), String> {
    let text = String::from_utf8_lossy(body).to_string();

    state.push_clipboard(text.clone());

    core_clipboard::set_clipboard_text(text);

    Ok(())
}

pub fn handle_window_show(state: &Arc<AppState>) -> Result<(), String> {
    state.set_window_visible(true);
    Ok(())
}

pub fn handle_window_hide(state: &Arc<AppState>) -> Result<(), String> {
    state.set_window_visible(false);
    Ok(())
}

pub fn handle_get_visible(state: &Arc<AppState>) -> Vec<u8> {
    let visible = state.is_window_visible();
    serde_json::to_vec(&visible).unwrap_or_default()
}

pub fn handle_clear(state: &Arc<AppState>) -> Result<(), String> {
    state.clear_history();
    Ok(())
}
use std::sync::Arc;
use crate::state::AppState;
use crate::models::PasteBridgeError;
use crate::clipboard as core_clipboard;

/// FFI 友好的 callback 集合: 桌面端用 `AppState` 适配,移动端用 Kotlin/Swift 实现。
/// 抽离出来之后,移动端不需要把整个 `AppState` 暴露给 UniFFI,
/// 只需要把同步相关的 4 个回调上送。
#[uniffi::export(callback_interface)]
pub trait ClipboardApiCallback: Send + Sync {
    /// 返回最近 N 条历史的 JSON 字节流 (与 `get_history` 序列化一致)。
    fn get_history_json(&self) -> Vec<u8>;
    /// 收到对端推送的剪贴板内容,本地侧负责入历史 + 写到系统剪贴板。
    fn on_remote_copy(&self, text: String) -> Result<(), PasteBridgeError>;
    /// 通知对端「窗口可见性」,留给平台侧决定怎么响应 (如唤起前台 / 提示)。
    fn set_window_visible(&self, visible: bool);
    /// 查询当前窗口可见性。
    fn is_window_visible(&self) -> bool;
}

pub fn handle_get_history(state: &Arc<AppState>) -> Vec<u8> {
    let history = state.get_history(false);
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

// ===== 移动端用的 callback-based 实现 =====
//
// 桌面端走 `Arc<AppState>` 旧路径,这里给移动端一个平行的入口。

pub fn handle_get_history_cb(cb: &dyn ClipboardApiCallback) -> Vec<u8> {
    cb.get_history_json()
}

pub fn handle_copy_cb(cb: &dyn ClipboardApiCallback, body: &[u8]) -> Result<(), PasteBridgeError> {
    let text = String::from_utf8_lossy(body).to_string();
    cb.on_remote_copy(text)
}

pub fn handle_window_show_cb(cb: &dyn ClipboardApiCallback) -> Result<(), PasteBridgeError> {
    cb.set_window_visible(true);
    Ok(())
}

pub fn handle_window_hide_cb(cb: &dyn ClipboardApiCallback) -> Result<(), PasteBridgeError> {
    cb.set_window_visible(false);
    Ok(())
}

pub fn handle_get_visible_cb(cb: &dyn ClipboardApiCallback) -> Vec<u8> {
    let visible = cb.is_window_visible();
    serde_json::to_vec(&visible).unwrap_or_default()
}

pub fn handle_clear_cb(_cb: &dyn ClipboardApiCallback) -> Result<(), PasteBridgeError> {
    // 当前设计: clear 由本机 UI 触发,不在同步接口暴露。
    // 留接口占位,后续如需对端触发清空再加。
    Ok(())
}
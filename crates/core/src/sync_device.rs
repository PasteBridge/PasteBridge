//! 跨设备 clipboard sync 闭环。
//!
//! 跑在任意能拿到 `Arc<AppState>` 的 host (桌面 / 移动端 UI 线程)
//! 的调用方把任务丢到 `std::thread` 后, 调这个函数。
//!
//! 流程:
//! 1. **Pull**: GET `{addr}:{port}/clipboard/history` -> 解析 JSON
//!    -> 用 `content_hash` 与本地去重 -> 调 `state.push_clipboard` 落入本地。
//! 2. **Push**: 取本地最新一条文本, POST `{addr}:{port}/clipboard/copy`,
//!    对端走 `state.push_clipboard` 入历史 (含对端已有的 hash 去重)。
//!
//! `peer.addresses` 可能是多个 (mDNS 会把对端所有 NIC 都列上), 这里
//! 逐个尝试,首个能连上的 IP 视为有效。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::discovery::DiscoveredPeer;
use crate::models::{ClipboardItem, ContentType};
use crate::state::AppState;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// 单次 sync 结果,用于驱动 UI 反馈 (toast / 状态条 / 计数)。
#[derive(Debug, Clone, uniffi::Record)]
pub struct SyncReport {
    /// 这次 sync 的对端 device_id。
    pub peer_device_id: String,
    /// 拉到了多少条 (去重前)。
    pub pulled_total: u64,
    /// 实际新增了多少条 (按 `content_hash` 去重后)。
    pub pulled_added: u64,
    /// 推了几条 (0 = 本地没有文本,或本端最新文本 hash 已存在于对端)。
    pub pushed: u64,
    /// 任意一方向失败时的错误描述, 没出错时为 None。
    pub error: Option<String>,
}

impl SyncReport {
    pub fn ok(peer_device_id: String, pulled_total: u64, pulled_added: u64, pushed: u64) -> Self {
        Self {
            peer_device_id,
            pulled_total,
            pulled_added,
            pushed,
            error: None,
        }
    }
}

/// 同步入口。整段跑在调用方传进来的 `std::thread` 里, 不阻塞 Slint 主循环。
pub fn sync_with_device(state: Arc<AppState>, peer: &DiscoveredPeer) -> SyncReport {
    if peer.addresses.is_empty() {
        return SyncReport {
            peer_device_id: peer.device_id.clone(),
            pulled_total: 0,
            pulled_added: 0,
            pushed: 0,
            error: Some("peer has no address".into()),
        };
    }

    let mut last_err: Option<String> = None;
    for addr in &peer.addresses {
        let base = format!("http://{}:{}", addr, peer.port);
        match sync_via_base(&state, &peer.device_id, &base) {
            Ok(report) => return report,
            Err(e) => {
                eprintln!("[sync] {} failed: {}; trying next address", base, e);
                last_err = Some(e);
            }
        }
    }

    SyncReport {
        peer_device_id: peer.device_id.clone(),
        pulled_total: 0,
        pulled_added: 0,
        pushed: 0,
        error: last_err,
    }
}

fn sync_via_base(
    state: &Arc<AppState>,
    peer_device_id: &str,
    base: &str,
) -> Result<SyncReport, String> {
    // ===== 0. 先记住本地最新文本 =====
    // 必须在 pull 之前抓, 否则 pull 进来的对端条目会变成"最新",
    // 把刚刚拉到的对端文本又原样推回去 (或被对端去重为空操作)。
    let latest_local_text = state
        .get_history(false)
        .into_iter()
        .find(|it| matches!(it.content_type, ContentType::Text))
        .and_then(|it| it.content_text)
        .filter(|t| !t.is_empty());

    // ===== 1. Pull =====
    let history_url = format!("{}/clipboard/history", base);
    let resp = ureq::get(&history_url)
        .timeout(HTTP_TIMEOUT)
        .call()
        .map_err(|e| format!("GET {}: {}", history_url, e))?;
    if resp.status() != 200 {
        return Err(format!("GET {} -> status {}", history_url, resp.status()));
    }
    let body = resp
        .into_string()
        .map_err(|e| format!("read body from {}: {}", history_url, e))?;

    let remote_items: Vec<ClipboardItem> =
        serde_json::from_str(&body).map_err(|e| format!("decode history: {}", e))?;

    let pulled_total = remote_items.len() as u64;

    // 用本地现有 hash 集合做 O(1) 去重, 避免重复 push_clipboard。
    let local_hashes: HashSet<String> = state
        .get_history(false)
        .into_iter()
        .map(|it| it.content_hash)
        .collect();

    let mut pulled_added: u64 = 0;
    for item in &remote_items {
        // 只同步文本, 图片体积大 + 协议未扩展, 跳过。
        if !matches!(item.content_type, ContentType::Text) {
            continue;
        }
        if local_hashes.contains(&item.content_hash) {
            continue;
        }
        let Some(text) = item.content_text.as_ref() else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        state.push_clipboard(text.clone());
        pulled_added += 1;
    }

    // ===== 2. Push =====
    let pushed = push_text_to(base, latest_local_text.as_deref())?;

    Ok(SyncReport::ok(
        peer_device_id.to_string(),
        pulled_total,
        pulled_added,
        pushed as u64,
    ))
}

/// 把指定文本 POST 到对端。
///
/// `text = None` 时表示本地没有可推内容, 跳过。
fn push_text_to(base: &str, text: Option<&str>) -> Result<usize, String> {
    let Some(text) = text else {
        return Ok(0);
    };
    if text.is_empty() {
        return Ok(0);
    }

    let copy_url = format!("{}/clipboard/copy", base);
    let resp = ureq::post(&copy_url)
        .timeout(HTTP_TIMEOUT)
        .send_string(text);

    match resp {
        Ok(r) if r.status() == 200 => Ok(1),
        Ok(r) => Err(format!("POST {} -> status {}", copy_url, r.status())),
        Err(e) => Err(format!("POST {}: {}", copy_url, e)),
    }
}

// ---------------------------------------------------------------------------
// UniFFI 导出层 — 不依赖 AppState，纯 HTTP pull + push
// ---------------------------------------------------------------------------

/// FFI-safe 跨设备同步入口.
///
/// Android 端调用此函数完成一次 sync:
///   1. Pull: GET `/clipboard/history` → 返回 JSON history
///   2. Push: POST `/clipboard/copy` (body = text_to_push)
///
/// 返回 [SyncReport]，Caller (Kotlin) 负责解析 history_json 并更新本地状态。
#[uniffi::export]
pub fn sync_with_peer(peer: DiscoveredPeer, text_to_push: String) -> SyncReport {
    if peer.addresses.is_empty() {
        return SyncReport {
            peer_device_id: peer.device_id.clone(),
            pulled_total: 0,
            pulled_added: 0,
            pushed: 0,
            error: Some("peer has no address".into()),
        };
    }

    let mut last_err: Option<String> = None;
    for addr in &peer.addresses {
        let base = format!("http://{}:{}", addr, peer.port);
        match sync_via_peer_http(&base, &peer.device_id, &text_to_push) {
            Ok(report) => return report,
            Err(e) => {
                eprintln!("[sync] {} failed: {}; trying next address", base, e);
                last_err = Some(e);
            }
        }
    }

    SyncReport {
        peer_device_id: peer.device_id.clone(),
        pulled_total: 0,
        pulled_added: 0,
        pushed: 0,
        error: last_err,
    }
}

fn sync_via_peer_http(base: &str, peer_device_id: &str, text_to_push: &str) -> Result<SyncReport, String> {
    // ===== 1. Pull =====
    let history_url = format!("{}/clipboard/history", base);
    let resp = ureq::get(&history_url)
        .timeout(HTTP_TIMEOUT)
        .call()
        .map_err(|e| format!("GET {}: {}", history_url, e))?;
    if resp.status() != 200 {
        return Err(format!("GET {} -> status {}", history_url, resp.status()));
    }
    let body = resp
        .into_string()
        .map_err(|e| format!("read body from {}: {}", history_url, e))?;

    let remote_items: Vec<ClipboardItem> =
        serde_json::from_str(&body).map_err(|e| format!("decode history: {}", e))?;

    let pulled_total = remote_items.len() as u64;

    // ===== 2. Push =====
    let pushed = push_text_to(base, Some(text_to_push))?;

    Ok(SyncReport::ok(
        peer_device_id.to_string(),
        pulled_total,
        pulled_total, // Android 端自己会去重，这里传总数
        pushed as u64,
    ))
}

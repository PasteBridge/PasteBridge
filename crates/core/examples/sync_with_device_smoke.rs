//! sync_with_device HTTP 闭环 smoke test
//!
//! 模拟两端 A (HTTP server) + B (sync_with_device caller) 在同进程内
//! 跑 `127.0.0.1:<port>`,验证:
//! 1. A 启动 HTTP server (`start_with_callbacks`),准备 3 条历史。
//! 2. B (空历史) 调 `sync_device::sync_with_device(A_peer)`:
//!    - pull: GET /clipboard/history -> 入 B 本地
//!    - push: B 本地最新 -> POST /clipboard/copy -> A 入历史
//! 3. 校验: B 拉到 3 条; A 收到 B push 的 1 条。
//!
//! 不依赖 mDNS 组播通不通,直接传 `DiscoveredPeer{ addresses: vec!["127.0.0.1"], port }` 即可。
//!
//! 跑法: cargo run --example sync_with_device_smoke

use std::sync::Arc;
use std::time::Duration;

use paste_bridge_core::api::ApiServer;
use paste_bridge_core::api::routes::ClipboardApiCallback;
use paste_bridge_core::discovery::DiscoveredPeer;
use paste_bridge_core::models::PasteBridgeError;
use paste_bridge_core::state::AppState;
use paste_bridge_core::sync_device::{sync_with_device, SyncReport};

const A_PORT: u16 = 28900;
const SEED_TEXT: &str = "hello-from-A";

fn make_state(label: &str) -> Arc<AppState> {
    let dir = std::env::temp_dir()
        .join(format!("pastebridge-sync-smoke-{}-{}", label, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    AppState::new(&dir, 100)
}

/// 把 `AppState` 包装成 `ClipboardApiCallback`, 让 HTTP server 路由
/// `/clipboard/history` / `/clipboard/copy` 走 `state` 的实际存储。
struct StateCallback(Arc<AppState>);

impl ClipboardApiCallback for StateCallback {
    fn get_history_json(&self) -> Vec<u8> {
        serde_json::to_vec(&self.0.get_history(false)).unwrap_or_default()
    }

    fn on_remote_copy(&self, text: String) -> Result<(), PasteBridgeError> {
        self.0.push_clipboard(text);
        Ok(())
    }

    fn set_window_visible(&self, _visible: bool) {}
    fn is_window_visible(&self) -> bool {
        true
    }
}

fn main() {
    eprintln!("[smoke-sync] A_PORT={}", A_PORT);

    // ====== A: server with 3 seeded entries ======
    let state_a = make_state("A");
    for i in 0..3 {
        state_a.push_clipboard(format!("{} #{}", SEED_TEXT, i));
    }
    let a_count_initial = state_a.get_history(false).len();
    eprintln!("[A] seeded count = {}", a_count_initial);
    assert_eq!(a_count_initial, 3);

    let server_a = ApiServer::new(A_PORT);
    server_a
        .start_with_callbacks(Box::new(StateCallback(state_a.clone())))
        .expect("A server start");
    std::thread::sleep(Duration::from_millis(300));

    // ====== B: client with 1 local entry (so push direction has something to send) ======
    let state_b = make_state("B");
    state_b.push_clipboard("from-B-1".to_string());
    state_b.push_clipboard("from-B-2".to_string());
    eprintln!("[B] initial count = {}", state_b.get_history(false).len());

    let peer = DiscoveredPeer {
        device_id: "smoke-A".to_string(),
        platform: "desktop".to_string(),
        addresses: vec!["127.0.0.1".to_string()],
        port: A_PORT,
        fullname: "smoke-A._pastebridge._tcp.local.".to_string(),
    };

    eprintln!("[B] calling sync_with_device(peer=A) ...");
    let report: SyncReport = sync_with_device(state_b.clone(), &peer);
    eprintln!("[B] report = pulled={} added={} pushed={} err={:?}",
        report.pulled_total, report.pulled_added, report.pushed, report.error);

    // ====== assertions ======
    let a_history = state_a.get_history(false);
    let b_history = state_b.get_history(false);

    eprintln!("[check] A.count={} B.count={}", a_history.len(), b_history.len());

    assert!(report.error.is_none(), "sync errored: {:?}", report.error);
    assert_eq!(report.pulled_total, 3, "expected 3 pulled total");
    assert_eq!(report.pulled_added, 3, "expected 3 pulled added");
    assert_eq!(report.pushed, 1, "expected 1 push (B's latest)");

    // B 应该包含 A 的 3 条 + 自己的 2 条
    assert_eq!(b_history.len(), 5, "B should have 5 entries (2 local + 3 from A)");
    let b_texts: Vec<String> = b_history
        .iter()
        .filter_map(|it| it.content_text.clone())
        .collect();
    for i in 0..3 {
        let want = format!("{} #{}", SEED_TEXT, i);
        assert!(b_texts.contains(&want), "B missing A's entry: {}", want);
    }

    // A 应该收到 B push 的 "from-B-2" (B 的最新一条)
    let a_texts: Vec<String> = a_history
        .iter()
        .filter_map(|it| it.content_text.clone())
        .collect();
    assert!(
        a_texts.iter().any(|t| t == "from-B-2"),
        "A should have received B's latest push; A texts = {:?}",
        a_texts
    );

    eprintln!("[smoke-sync] OK: pull + push closed loop verified");
    eprintln!("[smoke-sync] done");
}

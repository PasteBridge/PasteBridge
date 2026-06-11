//! Unicast fallback + HTTP sync 闭环测试.
//!
//! 用法:
//! ```
//! # 终端 1: 启 server
//! cargo run --example unicast_smoke -- server 28792
//! # 终端 2: 启 client (browse + unicast 探测 + sync)
//! cargo run --example unicast_smoke -- client 28793
//! ```
//!
//! server 启 PasteBridge HTTP API + 3 条种子.
//! client 启 Discovery 浏览 (mDNS + unicast fallback), 收到 peer 后调 sync_with_device,
//! 验证 pull + push 闭环.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use paste_bridge_core::api::ApiServer;
use paste_bridge_core::api::routes::ClipboardApiCallback;
use paste_bridge_core::discovery::{DiscoveredPeer, Discovery, DiscoveryListener};
use paste_bridge_core::models::PasteBridgeError;
use paste_bridge_core::state::AppState;
use paste_bridge_core::sync_device::sync_with_device;

struct StdoutListener {
    seen: Arc<Mutex<Vec<DiscoveredPeer>>>,
}

impl DiscoveryListener for StdoutListener {
    fn on_discovered(&self, peer: DiscoveredPeer) {
        let mut list = self.seen.lock().unwrap();
        if list.iter().any(|p| p.fullname == peer.fullname) {
            return;
        }
        eprintln!(
            "[smoke] DISCOVERED peer: device_id={} platform={} addrs={:?} port={} fullname={}",
            peer.device_id, peer.platform, peer.addresses, peer.port, peer.fullname
        );
        list.push(peer);
    }
    fn on_lost(&self, peer: DiscoveredPeer) {
        eprintln!("[smoke] LOST peer: fullname={}", peer.fullname);
    }
}

fn make_state(label: &str) -> Arc<AppState> {
    let dir = std::env::temp_dir()
        .join(format!("pastebridge-unicast-smoke-{}-{}", label, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    AppState::new(&dir, 100)
}

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

fn run_server(port: u16) {
    let state = make_state("server");
    for i in 0..3 {
        state.push_clipboard(format!("server-seed-{}", i));
    }
    eprintln!(
        "[server] seeded count = {}",
        state.get_history(false).len()
    );
    let server = ApiServer::new(port);
    server
        .start_with_callbacks(Box::new(StateCallback(state.clone())))
        .expect("server start");
    eprintln!("[server] listening on :{}, will exit after 90s", port);
    std::thread::sleep(Duration::from_secs(90));
}

fn run_client() {
    let state = make_state("client");
    state.push_clipboard("client-pushed-text".to_string());
    eprintln!(
        "[client] initial count = {}",
        state.get_history(false).len()
    );

    let discovery = Discovery::new().expect("Discovery::new");
    discovery
        .register(
            "smoke-client".to_string(),
            "desktop".to_string(),
            0,
            vec![],
        )
        .expect("register");

    let seen: Arc<Mutex<Vec<DiscoveredPeer>>> = Arc::new(Mutex::new(Vec::new()));
    let listener = StdoutListener {
        seen: seen.clone(),
    };
    discovery.browse(Box::new(listener)).expect("browse");

    eprintln!("[client] waiting up to 35s for peer discovery (mDNS + unicast fallback)...");
    for i in 0..35 {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(peer) = seen.lock().unwrap().first().cloned() {
            eprintln!("[client] got peer after {}s, calling sync_with_device", i);
            let report = sync_with_device(state.clone(), &peer);
            eprintln!(
                "[client] report: pulled={} added={} pushed={} err={:?}",
                report.pulled_total, report.pulled_added, report.pushed, report.error
            );
            eprintln!(
                "[client] final state count = {}",
                state.get_history(false).len()
            );
            return;
        }
    }
    eprintln!("[client] TIMEOUT: no peer discovered in 35s");
    discovery.shutdown();
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "client".to_string());
    match mode.as_str() {
        "server" => {
            let port: u16 = args
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(28792);
            run_server(port);
        }
        "client" => run_client(),
        _ => panic!("usage: unicast_smoke (server <port> | client)"),
    }
}

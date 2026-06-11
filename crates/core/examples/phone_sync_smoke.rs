//! Directly sync the local state with a known peer at a fixed host:port.
//! Bypasses mDNS / unicast discovery so we can verify the sync_with_device
//! HTTP path itself is healthy.

use std::sync::Arc;
use std::time::Duration;

use paste_bridge_core::api::routes::ClipboardApiCallback;
use paste_bridge_core::discovery::DiscoveredPeer;
use paste_bridge_core::models::PasteBridgeError;
use paste_bridge_core::state::AppState;
use paste_bridge_core::sync_device::sync_with_device;

struct NoopCallback(Arc<AppState>);
impl ClipboardApiCallback for NoopCallback {
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

fn make_state(label: &str) -> Arc<AppState> {
    let dir = std::env::temp_dir()
        .join(format!("pastebridge-phone-smoke-{}-{}", label, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    AppState::new(&dir, 100)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| "10.239.38.89:18792".to_string());

    let state = make_state("client");
    state.push_clipboard("client-seed-1".to_string());
    state.push_clipboard("client-seed-2".to_string());
    eprintln!("[phone-smoke] local state count = {}", state.get_history(false).len());

    let ip: std::net::Ipv4Addr = target
        .split(':')
        .next()
        .and_then(|s| s.parse().ok())
        .expect("bad host");
    let port: u16 = target
        .split(':')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("bad port");
    let peer = DiscoveredPeer {
        fullname: format!("direct-{}.local.", target),
        device_id: format!("direct-{}", target),
        platform: "android".to_string(),
        addresses: vec![std::net::IpAddr::V4(ip).to_string()],
        port,
    };

    eprintln!("[phone-smoke] calling sync_with_device against {}", target);
    let report = sync_with_device(state.clone(), &peer);
    eprintln!(
        "[phone-smoke] report: peer={} pulled={} added={} pushed={} err={:?}",
        report.peer_device_id, report.pulled_total, report.pulled_added, report.pushed, report.error
    );
    eprintln!(
        "[phone-smoke] final local state count = {}",
        state.get_history(false).len()
    );
    let hist = state.get_history(false);
    for (i, e) in hist.iter().take(8).enumerate() {
        eprintln!("[phone-smoke] #{}: id={} text={:?}", i, e.id, e.content_text);
    }
    std::thread::sleep(Duration::from_millis(100));
}

//! Unicast-only discovery that scans the full subnet and lists every peer it finds,
//! including 10.239.38.89 (the Android phone). Bypasses mDNS so we can isolate the
//! unicast fallback path.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use paste_bridge_core::discovery::{DiscoveredPeer, Discovery, DiscoveryListener};
use paste_bridge_core::models::PasteBridgeError;
use paste_bridge_core::state::AppState;
use paste_bridge_core::sync_device::sync_with_device;

struct CollectingListener {
    seen: Arc<Mutex<Vec<DiscoveredPeer>>>,
}

impl DiscoveryListener for CollectingListener {
    fn on_discovered(&self, peer: DiscoveredPeer) {
        let mut list = self.seen.lock().unwrap();
        if !list.iter().any(|p| p.fullname == peer.fullname) {
            eprintln!(
                "[scan] DISCOVERED: fullname={} addrs={:?} port={} device_id={} platform={}",
                peer.fullname, peer.addresses, peer.port, peer.device_id, peer.platform
            );
            list.push(peer);
        }
    }
    fn on_lost(&self, peer: DiscoveredPeer) {
        eprintln!("[scan] LOST: fullname={}", peer.fullname);
    }
}

fn main() {
    let state = Arc::new(AppState::new(
        &std::env::temp_dir().join(format!("unicast-full-{}", std::process::id())),
        100,
    ));
    state.push_clipboard("unicast-full-seed".to_string());

    let discovery = Discovery::new().expect("Discovery::new");
    discovery
        .register(
            "unicast-full".to_string(),
            "desktop".to_string(),
            0,
            vec![],
        )
        .expect("register");

    let seen: Arc<Mutex<Vec<DiscoveredPeer>>> = Arc::new(Mutex::new(Vec::new()));
    let listener = CollectingListener {
        seen: seen.clone(),
    };
    discovery.browse(Box::new(listener)).expect("browse");

    eprintln!("[scan] waiting 8s for discovery + unicast scan...");
    for i in 0..8 {
        std::thread::sleep(Duration::from_secs(1));
        let list = seen.lock().unwrap();
        let phone = list
            .iter()
            .find(|p| p.addresses.iter().any(|a| a.starts_with("10.239.38.89")));
        if phone.is_some() {
            eprintln!("[scan] found phone peer after {}s, calling sync_with_device", i + 1);
            let peer = phone.unwrap().clone();
            drop(list);
            let report = sync_with_device((*state).clone(), &peer);
            eprintln!(
                "[scan] report: peer={} pulled={} added={} pushed={} err={:?}",
                report.peer_device_id, report.pulled_total, report.pulled_added, report.pushed, report.error
            );
            eprintln!(
                "[scan] final state count = {}",
                state.get_history(false).len()
            );
            discovery.shutdown();
            return;
        }
    }
    eprintln!("[scan] TIMEOUT: phone not discovered. seen peers:");
    let list = seen.lock().unwrap();
    for p in list.iter() {
        eprintln!(
            "  fullname={} addrs={:?} port={} platform={}",
            p.fullname, p.addresses, p.port, p.platform
        );
    }
    discovery.shutdown();
}

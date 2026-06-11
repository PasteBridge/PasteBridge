//! mDNS 互通 smoke test。
//!
//! 用法:
//! ```
//! cargo run --example mdns_smoke -- <device_id> <platform> <port>
//! # 在另一个终端:
//! cargo run --example mdns_smoke -- <other_device_id> <other_platform> <other_port>
//! ```
//!
//! 两个进程会互相发现对方 `_pastebridge._tcp.local.` 服务,本进程触发
//! `on_discovered` 回调时把对方信息打印到 stderr。
//!
//! 这份 smoke test 走的是与 Android UniFFI 路径完全一致的
//! [`paste_bridge_core::discovery::Discovery`] 代码,任何 mdns-sd 的协议
//! bug 都会同时影响 Android 与本测试。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mdns_sd::DaemonEvent;
use paste_bridge_core::discovery::{DiscoveredPeer, Discovery, DiscoveryListener};

struct StdoutListener {
    seen: Arc<Mutex<Vec<DiscoveredPeer>>>,
}

impl DiscoveryListener for StdoutListener {
    fn on_discovered(&self, peer: DiscoveredPeer) {
        let mut list = self.seen.lock().unwrap();
        if list.iter().any(|p| p.fullname == peer.fullname) {
            eprintln!(
                "[smoke] duplicate, skip: fullname={} device_id={}",
                peer.fullname, peer.device_id
            );
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

fn main() {
    let mut args = std::env::args().skip(1);
    let device_id = args.next().unwrap_or_else(|| "smoke-device-a".to_string());
    let platform = args.next().unwrap_or_else(|| "desktop".to_string());
    let port: u16 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18792);

    eprintln!(
        "[smoke] starting: device_id={} platform={} port={}",
        device_id, platform, port
    );

    let discovery = Discovery::new().expect("Discovery::new");
    discovery
        .register(
            device_id.clone(),
            platform.clone(),
            port,
            vec![],
        )
        .expect("register");

    // 接 monitor 通道把 daemon 内部错误也打出来,便于排查多接口/防火墙问题。
    let monitor = discovery
        .daemon_handle()
        .monitor()
        .expect("Failed to monitor");
    std::thread::spawn(move || {
        for ev in monitor.iter() {
            if let DaemonEvent::Error(e) = ev {
                eprintln!("[smoke] daemon error: {}", e);
            }
        }
    });

    let listener = StdoutListener {
        seen: Arc::new(Mutex::new(Vec::new())),
    };
    discovery.browse(Box::new(listener)).expect("browse");

    eprintln!("[smoke] browsing... sleep 15s then shutdown");
    std::thread::sleep(Duration::from_secs(15));

    discovery.shutdown();
    eprintln!("[smoke] done");
}

//! Standalone smoke test for the mDNS discovery module.
//!
//! 启动后会在局域网注册 `_pastebridge._tcp` 服务并开始浏览，
//! 任何其他 PasteBridge 实例（或任何标准 mDNS 浏览器，例如 `dns-sd -B`）都能看到本机。
//!
//! 运行方式：
//! ```sh
//! cargo run --example mdns_smoke -- --device-id my-test-device
//! ```
//!
//! 在另一台机器上同时运行同一个例子验证双向发现。
//! 退出方式：按 Ctrl+C —— Windows 下程序会被直接终止，由 Drop 自动反注册。

use std::env;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use paste_bridge_core::discovery::{Discovery, DiscoveredPeer};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let device_id = args
        .iter()
        .position(|a| a == "--device-id")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "smoke-test-device".to_string());

    eprintln!("[smoke] device_id = {}", device_id);

    let discovery = match Discovery::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[smoke] Failed to init discovery: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let addresses: Vec<String> = Vec::new();
    if let Err(e) = discovery.register(&device_id, "smoke", 18792, &addresses) {
        eprintln!("[smoke] register failed: {}", e);
        return ExitCode::FAILURE;
    }

    let found_count = Arc::new(AtomicUsize::new(0));
    let found_count_clone = found_count.clone();
    if let Err(e) = discovery.browse(move |peer: DiscoveredPeer| {
        let n = found_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!(
            "[smoke] #{} discovered: device_id={} platform={} addrs={:?} port={} fullname={}",
            n, peer.device_id, peer.platform, peer.addresses, peer.port, peer.fullname
        );
    }) {
        eprintln!("[smoke] browse failed: {}", e);
        return ExitCode::FAILURE;
    }

    eprintln!("[smoke] running for 30s, Ctrl+C to exit early");
    for _ in 0..150 {
        thread::sleep(Duration::from_millis(200));
    }

    eprintln!(
        "[smoke] exit, total peers discovered: {}",
        found_count.load(Ordering::SeqCst)
    );
    discovery.shutdown();
    ExitCode::SUCCESS
}
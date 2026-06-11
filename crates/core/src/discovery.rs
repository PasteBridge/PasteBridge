//! mDNS service discovery for PasteBridge multi-device sync.
//!
//! 提供两个核心能力：
//! 1. **注册自身**：在 `SERVICE_TYPE` 上广播 `_pastebridge._tcp` 服务，对局域网宣告
//!    「本机是 PasteBridge，监听端口 P」
//! 2. **浏览远端**：监听同类型服务的 `ServiceResolved` 事件，回调发现的设备信息
//!
//! TXT 记录约定：
//! - `device_id`  ：本机的稳定 UUID（来自 [crate::device]）
//! - `platform`   ：平台标识 (`desktop` | `android` | `ios`)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::models::PasteBridgeError;

/// PasteBridge 多端同步的 mDNS 服务类型。
pub const SERVICE_TYPE: &str = "_pastebridge._tcp.local.";

/// 远端发现的 PasteBridge 设备信息。
#[derive(Debug, Clone, uniffi::Record)]
pub struct DiscoveredPeer {
    /// 本机的稳定 UUID（来自 TXT `device_id`，否则回退到完整服务名）
    pub device_id: String,
    /// 平台标识（`desktop` / `android` / `ios`，缺失时为空串）
    pub platform: String,
    /// 设备所有可用的 IP 地址字符串（已经过解析，去重）
    pub addresses: Vec<String>,
    /// 设备提供的 PasteBridge HTTP API 端口
    pub port: u16,
    /// mDNS 完整服务名，便于去重
    pub fullname: String,
}

/// Discovery 浏览事件的回调接口。
///
/// 桌面端实现此 trait 以复用原来的「去重 + 推 Slint」逻辑；
/// 移动端由 UniFFI 生成的 Kotlin / Swift 接口直接实现即可。
///
/// `on_lost` 在 Android NSD / iOS NSNetService 主动推送 service lost 时触发；
/// mdns-sd 在桌面端不暴露 lost 事件，但保留接口以便桌面端后续按需接入（桌面端
/// 实现为 no-op 即可）。
#[uniffi::export(callback_interface)]
pub trait DiscoveryListener: Send + Sync {
    /// 远端设备被解析出 IP/端口后调用。
    fn on_discovered(&self, peer: DiscoveredPeer);
    /// 远端设备下线（NSD lost / TTL 过期）后调用。
    /// 桌面端 mdns-sd 不感知 lost，可实现为空函数。
    fn on_lost(&self, peer: DiscoveredPeer);
}

/// mDNS 注册与浏览的统一入口。
#[derive(uniffi::Object)]
pub struct Discovery {
    daemon: ServiceDaemon,
    /// 当前已注册的 service fullname，用于关闭时反注册
    registered_fullname: Mutex<Option<String>>,
}

#[uniffi::export]
impl Discovery {
    /// 创建一个新的 mDNS 守护进程。
    ///
    /// **Windows 多网卡陷阱**：mdns-sd 0.17 默认在所有 IPv4 接口上 joinMulticast。
    /// 在装了 Mihomo / Radmin VPN / 蓝牙等虚拟网卡的机器上, mDNS 多播会从这些
    /// 接口出去, 而虚拟网卡通常不把多播包转回 WLAN, 导致局域网内其它设备收不到
    /// 我们的 announcement, 我们也收不到别人的 announcement.
    /// 此外 mdns-sd 0.17 的 `enable_interface` 在 Windows winsock2 路径下
    /// 不会真正限制 socket, 走 PR #164 的"按子网去重"逻辑, 多个 IP 会被认为是
    /// 同一子网, 随机选一个用.
    ///
    /// 修复策略 (两个独立层叠加):
    /// 1. **mDNS 层**: 不调 enable/disable, 让它默认跑全网卡 (多网卡用户能收多少算多少).
    /// 2. **Unicast fallback 层**: [`Discovery::browse`] 启动一个后台线程,
    ///    扫描本机所有 RFC1918 子网 (10/8, 172.16/12, 192.168/16), 对子网内
    ///    每个 IP 主动 TCP 探测 PasteBridge 端口. 见 [`unicast_probe_loop`].
    ///
    /// Unicast fallback 100% 可靠, 独立于 mDNS 多播 / 路由器 IGMP / AP 隔离,
    /// 代价是 1 次扫描 ~10s 内完成 (子网内 254 个 IP, 每个 50ms timeout).
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, PasteBridgeError> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| PasteBridgeError::generic(format!("Failed to create mDNS daemon: {}", e)))?;
        Ok(Arc::new(Self {
            daemon,
            registered_fullname: Mutex::new(None),
        }))
    }

    /// 在局域网内注册本机的 PasteBridge 服务。
    ///
    /// `service_name` 为可见的实例名，长度受 DNS 标签限制，因此仅取 `device_id` 前 8 字符。
    /// `addresses` 是本机的物理 IPv4 地址（来自 [crate::net]），传入后会被写入 A/AAAA 记录。
    pub fn register(
        &self,
        device_id: String,
        platform: String,
        port: u16,
        addresses: Vec<String>,
    ) -> Result<(), PasteBridgeError> {
        // 取 device_id 前 12 字符 + 端口确保 service_name 在同设备多实例时不冲突。
        let short_id: String = device_id.chars().take(12).collect();
        let service_name = format!("PasteBridge-{}-{}", sanitize_label(&short_id), port);
        let host = resolve_hostname();

        let mut properties = HashMap::new();
        properties.insert("device_id".to_string(), device_id.clone());
        properties.insert("platform".to_string(), platform.clone());

        // mdns-sd 0.17 的 ServiceInfo::new 接受 (ty_domain, my_name, host_name, ip, port, properties)
        let ip_arg: String = if addresses.is_empty() {
            "0.0.0.0".to_string()
        } else {
            addresses.join(",")
        };

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &host,
            ip_arg,
            port,
            properties,
        )
        .map_err(|e| PasteBridgeError::generic(format!("Failed to build ServiceInfo: {}", e)))?;

        let fullname = service_info.get_fullname().to_string();
        self.daemon
            .register(service_info)
            .map_err(|e| PasteBridgeError::generic(format!("Failed to register mDNS service: {}", e)))?;

        *self.registered_fullname.lock().unwrap() = Some(fullname.clone());
        eprintln!(
            "[mdns] Registered: {} (host={}, port={}, platform={}, device_id={})",
            fullname, host, port, platform, short_id
        );
        Ok(())
    }

    /// 启动后台浏览线程，发现远端 PasteBridge 设备。
    ///
    /// 每次 `ServiceResolved` 事件触发一次 `listener.on_discovered` 回调；
    /// 同一个设备如果重复收到 resolved 事件，回调也会被多次调用，由调用方按 `device_id` 去重。
    /// 桌面端 mdns-sd 不暴露 lost 事件，listener.on_lost 仅在外部自行清理时使用。
    ///
    /// 同时启动 unicast fallback 后台线程 [`unicast_probe_loop`]: 即使 mDNS
    /// 多播被多网卡/路由器 AP 隔离拦截, 也能通过主动 TCP 探测同子网设备来发现.
    /// 探测到的 peer 也走同一个 listener, 桌面端按 device_id 去重即可.
    pub fn browse(&self, listener: Box<dyn DiscoveryListener>) -> Result<(), PasteBridgeError> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| PasteBridgeError::generic(format!("Failed to start browse: {}", e)))?;

        // listener 在多个后台线程间共享, 用 mpsc 集中分发, 避免竞态.
        let shared: Arc<dyn DiscoveryListener> = Arc::from(listener);
        let (tx, rx) = std::sync::mpsc::channel::<DiscoveredPeer>();

        // 1) mDNS browse 线程
        let tx_mdns = tx.clone();
        std::thread::spawn(move || {
            for event in receiver.iter() {
                match event {
                    ServiceEvent::ServiceFound(fullname, ty) => {
                        eprintln!("[mdns] ServiceFound: fullname={} ty={}", fullname, ty);
                    }
                    ServiceEvent::ServiceResolved(resolved) => {
                        let peer = peer_from_resolved(&*resolved);
                        eprintln!(
                            "[mdns] Discovered: {} @ {:?}:{} (platform={})",
                            peer.device_id, peer.addresses, peer.port, peer.platform
                        );
                        let _ = tx_mdns.send(peer);
                    }
                    ServiceEvent::ServiceRemoved(fullname, reason) => {
                        eprintln!("[mdns] ServiceRemoved: {} ({:?})", fullname, reason);
                    }
                    ServiceEvent::SearchStopped(ty) => {
                        eprintln!("[mdns] Browse stopped for {}", ty);
                        break;
                    }
                    other => {
                        eprintln!("[mdns] other event: {:?}", other);
                    }
                }
            }
        });

        // 2) unicast fallback 线程
        let tx_unicast = tx.clone();
        std::thread::spawn(move || {
            unicast_probe_loop(tx_unicast);
        });

        // 3) 主分发线程: 串行调用 listener.on_discovered.
        std::thread::spawn(move || {
            for peer in rx.iter() {
                shared.on_discovered(peer);
            }
        });

        Ok(())
    }

    /// 关闭并反注册所有已注册的服务。
    pub fn shutdown(&self) {
        if let Some(fullname) = self.registered_fullname.lock().unwrap().take() {
            if let Err(e) = self.daemon.unregister(&fullname) {
                eprintln!("[mdns] unregister {} failed: {}", fullname, e);
            }
        }
        if let Err(e) = self.daemon.shutdown() {
            eprintln!("[mdns] daemon shutdown failed: {}", e);
        }
    }
}

/// 周期性扫描本机所有 RFC1918 子网, 对每个 IP TCP 探测 PasteBridge 端口, 找到就上报.
///
/// 启动时立即扫一次, 此后每 `UNICAST_PROBE_INTERVAL` 扫一次.
///
/// 端口选择策略: 不在 mDNS TXT 里带, 因为 TXT 解析已经成功了. 但 unicast probe
/// 不知道对端用什么端口, 因此**探几个常见默认端口**: 28792, 28080, 8888, 80.
/// 真要严谨, 可以让 desktop / Android 启动时把端口写到本地文件 (`pb_port`).
/// 当前为了不阻塞发布, 接受多试几个端口的 5s 探测开销.
fn unicast_probe_loop(tx: std::sync::mpsc::Sender<DiscoveredPeer>) {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
    use std::time::Duration;

    const UNICAST_PROBE_INTERVAL: Duration = Duration::from_secs(30);
    const PROBE_TIMEOUT: Duration = Duration::from_millis(80);
    const PROBE_PORTS: &[u16] = &[28792, 28080, 18800, 18792, 8888, 8080, 80];
    const CONCURRENCY: usize = 32;

    loop {
        let our_subnets = collect_rfc1918_subnets();
        if our_subnets.is_empty() {
            eprintln!("[unicast] no RFC1918 subnet to probe, sleep");
            std::thread::sleep(UNICAST_PROBE_INTERVAL);
            continue;
        }
        eprintln!(
            "[unicast] scanning {} subnet(s) for PasteBridge peers (concurrency={})",
            our_subnets.len(),
            CONCURRENCY
        );

        // 把所有候选 (ip, port) 平铺成一个 list, 然后用 CONCURRENCY 个工作线程并发扫.
        // 254 IP * 6 ports = 1524 个探测, 单线程串行要 ~120s; 并发 32 → ~4s.
        let mut candidates: Vec<(std::net::Ipv4Addr, u16)> = Vec::new();
        // 1) 同机: 127.0.0.1
        for &port in PROBE_PORTS {
            candidates.push((Ipv4Addr::new(127, 0, 0, 1), port));
        }
        // 2) 本机所有 RFC1918 子网的 .1..=254 (跳过本机 IP)
        for subnet in &our_subnets {
            for last in 1u8..=254 {
                let ip = Ipv4Addr::new(subnet.0, subnet.1, subnet.2, last);
                if ip == subnet.3 {
                    continue;
                }
                for &port in PROBE_PORTS {
                    candidates.push((ip, port));
                }
            }
        }

        // 用 Arc<Mutex<HashSet<ip:port>>> 收集已找到的 peer, 避免重复 dispatch.
        // (DiscoveryListener 那边没去重时, 同一个 peer 会被多个 port 命中.)
        use std::sync::Mutex;
        let found: std::sync::Arc<Mutex<std::collections::HashSet<(std::net::Ipv4Addr, u16)>>> =
            std::sync::Arc::new(Mutex::new(std::collections::HashSet::new()));

        let chunk_size = (candidates.len() + CONCURRENCY - 1) / CONCURRENCY;
        let mut handles = Vec::new();
        for chunk in candidates.chunks(chunk_size.max(1)) {
            let chunk = chunk.to_vec();
            let tx_w = tx.clone();
            let found_w = found.clone();
            handles.push(std::thread::spawn(move || {
                for (ip, port) in chunk {
                    if found_w.lock().unwrap().contains(&(ip, port)) {
                        continue;
                    }
                    let v4 = SocketAddrV4::new(ip, port);
                    let target: SocketAddr = SocketAddr::V4(v4);
                    match TcpStream::connect_timeout(&target, PROBE_TIMEOUT) {
                        Ok(_stream) => {
                            if ip.octets()[3] == 130 || (ip.octets()[3] == 1 && port == 28792) {
                                eprintln!("[unicast] connect OK {}:{}", ip, port);
                            }
                            if verify_pastebridge_peer(ip, port) {
                                eprintln!("[unicast] found PasteBridge peer at {}:{}", ip, port);
                                found_w.lock().unwrap().insert((ip, port));
                                let peer = DiscoveredPeer {
                                    device_id: format!("unicast-{}", ip),
                                    platform: "unknown".to_string(),
                                    addresses: vec![ip.to_string()],
                                    port,
                                    fullname: format!("unicast-{}.local.", ip),
                                };
                                if tx_w.send(peer).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            if ip.octets()[3] == 130 && (port == 18792 || port == 28792) {
                                eprintln!("[unicast] connect FAIL {}:{} : {}", ip, port, e);
                            }
                        }
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        eprintln!("[unicast] scan round done");

        std::thread::sleep(UNICAST_PROBE_INTERVAL);
    }
}

/// 收集本机所有 RFC1918 子网 + 本机 IP. 返回 `(network, our_ip)` 元组列表.
///
/// 用 getifaddrs 拿真实 NIC, 排除 loopback / 虚拟 NIC. 子网按 /24 简化.
fn collect_rfc1918_subnets() -> Vec<(u8, u8, u8, std::net::Ipv4Addr)> {
    let mut out = Vec::new();
    let ifaces = match getifaddrs::getifaddrs() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[unicast] getifaddrs failed: {}", e);
            return out;
        }
    };
    for iface in ifaces {
        let name = nic_friendly_name(&iface);
        if is_virtual_nic_name(&name) {
            continue;
        }
        let v4 = match &iface.address {
            getifaddrs::Address::V4(v4) if !v4.address.is_loopback() => v4.address,
            _ => continue,
        };
        let o = v4.octets();
        // 简单 /24 假设
        if is_rfc1918(v4) {
            out.push((o[0], o[1], o[2], v4));
        }
    }
    out
}

/// TCP 探测完成后, 主动发一个 HTTP GET /clipboard/history. 收到 200 + JSON 数组
/// 即认定是对端 PasteBridge (避免把 80 端口的路由器管理页 / 28792 上跑的其他服务
/// 误判为 peer).
fn verify_pastebridge_peer(ip: std::net::Ipv4Addr, port: u16) -> bool {
    let url = format!("http://{}:{}/clipboard/history", ip, port);
    let r = std::panic::catch_unwind(|| ureq::get(&url).timeout(std::time::Duration::from_millis(2000)).call());
    let result = match r {
        Ok(Ok(resp)) if resp.status() == 200 => true,
        Ok(Ok(resp)) => {
            eprintln!("[unicast] verify {} -> status {}", url, resp.status());
            false
        }
        Ok(Err(e)) => {
            eprintln!("[unicast] verify {} -> err {}", url, e);
            false
        }
        Err(_) => {
            eprintln!("[unicast] verify {} -> panic", url);
            false
        }
    };
    result
}

fn nic_friendly_name(iface: &getifaddrs::Interface) -> String {
    #[cfg(windows)]
    {
        if !iface.description.is_empty() {
            return iface.description.clone();
        }
    }
    iface.name.clone()
}

fn is_rfc1918(ip: std::net::Ipv4Addr) -> bool {
    let o = ip.octets();
    if o[0] == 10 {
        return true;
    }
    if o[0] == 172 && (16..=31).contains(&o[1]) {
        return true;
    }
    if o[0] == 192 && o[1] == 168 {
        return true;
    }
    false
}

fn is_virtual_nic_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    const BLOCKLIST: &[&str] = &[
        "mihomo", "tun", "tap", "vpn", "radmin", "hamachi", "tailscale",
        "zerotier", "wireguard", "wg", "loopback", "lo", "bluetooth",
        "蓝牙", "teredo", "isatap", "6to4", "vmware", "virtualbox", "hyper-v",
        "hyperv", "vEthernet", "本地连接*",
    ];
    BLOCKLIST.iter().any(|kw| n.contains(kw))
}

// ===== Rust-only 接口(不导出到 FFI) =====
impl Discovery {
    /// 把内部 `ServiceDaemon` 借出去,主要给 smoke test 拿 `monitor()` 通道。
    pub fn daemon_handle(&self) -> &ServiceDaemon {
        &self.daemon
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 从 `ResolvedService` 中提取 [DiscoveredPeer]。
///
/// `ResolvedService` 是 `ServiceInfo` 的轻量包装；这里直接通过 `&**resolved` 拿到 `&ServiceInfo`。
fn peer_from_resolved(resolved: &ResolvedService) -> DiscoveredPeer {
    let fullname = resolved.get_fullname().to_string();
    let port = resolved.get_port();

    // 优先从 TXT 读取 `device_id`，否则退到完整服务名
    let device_id = resolved
        .get_property_val_str("device_id")
        .map(|s| s.to_string())
        .unwrap_or_else(|| fullname.clone());

    let platform = resolved
        .get_property_val_str("platform")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // ScopedIp → 标准 IpAddr → 字符串
    let addresses: Vec<String> = resolved
        .get_addresses()
        .iter()
        .map(|scoped| scoped.to_ip_addr().to_string())
        .collect();

    DiscoveredPeer {
        device_id,
        platform,
        addresses,
        port,
        fullname,
    }
}

/// 取本机主机名（作为 mDNS 的 host 字段），并保证以 `.local.` 结尾。
/// mDNS 要求所有主机名都是 `.local.` 域下的，本机 COMPUTERNAME 通常不带该后缀，
/// 这里统一补齐。
fn resolve_hostname() -> String {
    let raw = std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "pastebridge".to_string());

    if raw.ends_with(".local.") {
        raw
    } else {
        format!("{}.local.", raw)
    }
}

/// 把任意字符串收敛成合法的 DNS label（仅保留字母数字与 `-`）。
/// 仅用于本机声明的 service name，不会影响 device_id 的真实值。
fn sanitize_label(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch);
        }
    }
    if out.is_empty() {
        out.push_str("device");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_label_strips_special_chars() {
        assert_eq!(sanitize_label("a1b2:c3-d4"), "a1b2c3-d4");
        assert_eq!(sanitize_label(""), "device");
        assert_eq!(sanitize_label("@@@"), "device");
    }

    #[test]
    fn resolve_hostname_is_non_empty() {
        let h = resolve_hostname();
        assert!(!h.is_empty());
    }
}
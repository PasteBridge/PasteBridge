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
use std::sync::Mutex;

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

/// PasteBridge 多端同步的 mDNS 服务类型。
pub const SERVICE_TYPE: &str = "_pastebridge._tcp.local.";

/// 远端发现的 PasteBridge 设备信息。
#[derive(Debug, Clone)]
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

/// mDNS 注册与浏览的统一入口。
pub struct Discovery {
    daemon: ServiceDaemon,
    /// 当前已注册的 service fullname，用于关闭时反注册
    registered_fullname: Mutex<Option<String>>,
}

impl Discovery {
    /// 创建一个新的 mDNS 守护进程。
    pub fn new() -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;
        Ok(Self {
            daemon,
            registered_fullname: Mutex::new(None),
        })
    }

    /// 在局域网内注册本机的 PasteBridge 服务。
    ///
    /// `service_name` 为可见的实例名，长度受 DNS 标签限制，因此仅取 `device_id` 前 8 字符。
    /// `addresses` 是本机的物理 IPv4 地址（来自 [crate::net]），传入后会被写入 A/AAAA 记录。
    pub fn register(
        &self,
        device_id: &str,
        platform: &str,
        port: u16,
        addresses: &[String],
    ) -> Result<(), String> {
        // 取 device_id 前 12 字符 + 端口确保 service_name 在同设备多实例时不冲突。
        let short_id: String = device_id.chars().take(12).collect();
        let service_name = format!("PasteBridge-{}-{}", sanitize_label(&short_id), port);
        let host = resolve_hostname();

        let mut properties = HashMap::new();
        properties.insert("device_id".to_string(), device_id.to_string());
        properties.insert("platform".to_string(), platform.to_string());

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
        .map_err(|e| format!("Failed to build ServiceInfo: {}", e))?;

        let fullname = service_info.get_fullname().to_string();
        self.daemon
            .register(service_info)
            .map_err(|e| format!("Failed to register mDNS service: {}", e))?;

        *self.registered_fullname.lock().unwrap() = Some(fullname.clone());
        eprintln!(
            "[mdns] Registered: {} (host={}, port={}, platform={}, device_id={})",
            fullname, host, port, platform, short_id
        );
        Ok(())
    }

    /// 启动后台浏览线程，发现远端 PasteBridge 设备。
    ///
    /// 每次 `ServiceResolved` 事件触发一次 `on_discovered` 回调；同一个设备如果
    /// 重复收到 resolved 事件，回调也会被多次调用，由调用方按 `device_id` 去重。
    pub fn browse<F>(&self, on_discovered: F) -> Result<(), String>
    where
        F: Fn(DiscoveredPeer) + Send + 'static,
    {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("Failed to start browse: {}", e))?;

        std::thread::spawn(move || {
            for event in receiver.iter() {
                match event {
                    ServiceEvent::ServiceResolved(resolved) => {
                        let peer = peer_from_resolved(&*resolved);
                        eprintln!(
                            "[mdns] Discovered: {} @ {:?}:{} (platform={})",
                            peer.device_id, peer.addresses, peer.port, peer.platform
                        );
                        on_discovered(peer);
                    }
                    ServiceEvent::SearchStopped(ty) => {
                        eprintln!("[mdns] Browse stopped for {}", ty);
                        break;
                    }
                    _ => {}
                }
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
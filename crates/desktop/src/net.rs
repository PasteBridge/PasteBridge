//! 本机网络工具: 枚举所有非回环 IPv4 地址。
//!
//! 用于在同步面板(Sync / SharePanel)左下角显示本机所有可用 IP,方便用户告知远端设备
//! "应当连接哪个地址"。仅返回 IPv4,过滤 127.0.0.0/8 之外的物理/虚拟接口地址。

use if_addrs::{get_if_addrs, IfAddr};

/// 获取本机所有 IPv4 地址(已过滤回环),按接口名稳定排序。
/// 失败时返回空列表(不向 UI 抛错)。
pub fn list_local_ipv4() -> Vec<String> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if let IfAddr::V4(v4) = iface.addr {
                let ip = v4.ip.to_string();
                // 过滤回环地址(127.0.0.0/8)
                if ip.starts_with("127.") {
                    continue;
                }
                out.push((iface.name, ip));
            }
        }
    }
    // 稳定排序: 先按接口名,再按 IP
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out.into_iter().map(|(_, ip)| ip).collect()
}

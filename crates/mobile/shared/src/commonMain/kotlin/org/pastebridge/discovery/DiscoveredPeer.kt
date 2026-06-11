package org.pastebridge.discovery

/**
 * 远端发现的 PasteBridge 设备信息。
 *
 * 与 Rust 端 [paste_bridge_core::discovery::DiscoveredPeer] 字段一一对应。
 */
data class DiscoveredPeer(
    /** 本机的稳定 UUID (来自 TXT `device_id`，否则回退到完整服务名) */
    val deviceId: String,
    /** 平台标识 (`desktop` / `android` / `ios`，缺失时为空串) */
    val platform: String,
    /** 设备所有可用的 IP 地址字符串 */
    val addresses: List<String>,
    /** 设备提供的 PasteBridge HTTP API 端口 */
    val port: Int,
    /** mDNS 完整服务名，便于去重 */
    val fullname: String,
)
package org.pastebridge.discovery

/**
 * 多端同步的 mDNS / NSD 服务发现抽象。
 *
 * - [register]: 在局域网内宣告本机是 PasteBridge，并附带 device_id / platform 等 TXT 元数据
 * - [browse]: 异步浏览局域网所有 PasteBridge 实例，每次发现一个就回调 [onDiscovered]
 * - [stop]: 释放底层资源（反注册服务 + 取消 browse 监听）
 *
 * 协议约定（与 Rust 端 core/src/discovery.rs 保持一致）：
 * - 服务类型: `_pastebridge._tcp.local.`
 * - TXT 记录: `device_id=<uuid>`, `platform=android|ios|desktop`
 */
expect class DiscoveryService() {
    /**
     * 在局域网内注册本机的 PasteBridge 服务。
     *
     * @param deviceId 本机的稳定 UUID
     * @param platform 平台标识 (`android` 或 `ios`)
     * @param port PasteBridge HTTP API 端口
     */
    fun register(deviceId: String, platform: String, port: Int)

    /**
     * 启动后台浏览。每次发现远端 PasteBridge 设备时回调 [onDiscovered]。
     * 同一设备可能触发多次回调，由调用方按 deviceId 去重。
     *
     * @param onDiscovered 主线程分发的发现回调 (平台实现负责切线程)
     */
    fun browse(onDiscovered: (DiscoveredPeer) -> Unit)

    /** 释放所有资源。建议在 Activity / ViewController onDestroy 中调用。 */
    fun stop()
}
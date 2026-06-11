package org.pastebridge.discovery

/**
 * iOS 桩实现：返回 null（按用户要求先跳过 iOS 端实现）。
 */
actual class DiscoveryService actual constructor() {
    actual fun register(deviceId: String, platform: String, port: Int) {}
    actual fun browse(
        onDiscovered: (DiscoveredPeer) -> Unit,
        onLost: (DiscoveredPeer) -> Unit,
    ) {}
    actual fun stop() {}
}

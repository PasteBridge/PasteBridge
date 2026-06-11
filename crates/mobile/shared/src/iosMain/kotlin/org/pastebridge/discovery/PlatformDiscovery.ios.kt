package org.pastebridge.discovery

import androidx.compose.runtime.Composable

/**
 * iOS 桩实现：返回 null（NSNetService 适配暂未实现，按用户要求先跳过）。
 * 等待实现 NSNetService 注册 + NSNetServiceBrowser 浏览后替换为真实单例。
 */
@Composable
actual fun platformDiscovery(): DiscoveryService? = null

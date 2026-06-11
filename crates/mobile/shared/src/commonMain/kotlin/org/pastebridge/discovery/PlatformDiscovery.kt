package org.pastebridge.discovery

import androidx.compose.runtime.Composable

/**
 * 获取当前平台对应的 [DiscoveryService] 单例。
 *
 * - Android: 来自 Activity 已注入的 [DiscoveryServiceProvider]，若 Activity 未先调用
 *   [DiscoveryService.init] 则返回 null。
 * - iOS: 暂返回 null（待 NSNetService 适配完成）。
 *
 * 使用方无需关心 init 细节，未注入 Context 时本函数返回 null，UI 可降级展示。
 */
@Composable
expect fun platformDiscovery(): DiscoveryService?
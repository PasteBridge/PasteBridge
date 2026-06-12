package org.pastebridge.app

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform

/**
 * 触发与某个 peer 的一次同步。
 *
 * common 层只声明接口,真正的 FFI 调用（UniFFI syncWithPeer）放在 androidMain 实际实现里。
 * 桌面端走 Rust API,不需要这个 expect。
 *
 * 返回一个 Result<SyncOutcome> 字符串,UI 直接展示给用户。
 * SyncOutcome 描述"拉到了 N 条 / 推了 N 条 / 失败原因"。
 */
expect suspend fun syncWithPeerCommon(
    deviceId: String,
    platform: String,
    addresses: List<String>,
    port: Int,
    fullname: String,
): String

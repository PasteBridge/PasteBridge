package org.pastebridge.app

import android.os.Build
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.pastebridge.sync.SyncService
import uniffi.paste_bridge_core.DiscoveredPeer

class AndroidPlatform : Platform {
    override val name: String = "Android "
}

actual fun getPlatform(): Platform = AndroidPlatform()

/**
 * Android 端把 common 层 [org.pastebridge.discovery.DiscoveredPeer] 字段转发给
 * UniFFI 生成的 [uniffi.paste_bridge_core.DiscoveredPeer],然后调
 * [SyncService.syncWithPeer] 跑一次 sync。UniFFI 内部会 GET 对端 /clipboard/history
 * 拉 JSON,再 POST /clipboard/copy 把本机最新文本推过去。
 *
 * 整段跑在 IO 调度器,避免阻塞主线程。
 */
actual suspend fun syncWithPeerCommon(
    deviceId: String,
    platform: String,
    addresses: List<String>,
    port: Int,
    fullname: String,
): String = withContext(Dispatchers.IO) {
    val ctx = AppContext.applicationContext
        ?: return@withContext "错误: SyncService 未初始化 (MainActivity 还没跑)"
    val peer = DiscoveredPeer(
        deviceId = deviceId,
        platform = platform,
        addresses = addresses,
        port = port.toUShort(),
        fullname = fullname,
    )
    val report = try {
        SyncService.get(ctx).syncWithPeer(peer)
    } catch (e: Throwable) {
        return@withContext "同步失败: : "
    }
    val err = report.error
    if (err != null) "同步失败: " else {
        "✓ 同步完成\n" +
            "对端 \n" +
            "拉到  条 (新增 )\n" +
            "推送  条"
    }
}

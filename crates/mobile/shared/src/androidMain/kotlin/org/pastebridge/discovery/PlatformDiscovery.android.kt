package org.pastebridge.discovery

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

/**
 * Android 实际：从全局单例 [DiscoveryServiceProvider] 获取。
 * 若 Activity 尚未调用 init(context) 会得到 null。
 */
@Composable
actual fun platformDiscovery(): DiscoveryService? = LocalContext.current.let { ctx ->
    DiscoveryServiceProvider.tryGet(ctx)
}

/**
 * Android 专用单例，封装 Context 注入。
 * 由 MainActivity.onCreate 调用 init(context) 后即可全局使用。
 */
object DiscoveryServiceProvider {
    private val service = DiscoveryService()
    private var initialized = false

    fun init(context: android.content.Context) {
        if (initialized) return
        service.init(context.applicationContext)
        initialized = true
    }

    fun tryGet(context: android.content.Context): DiscoveryService {
        // 兜底:首次 Composable 访问时若未 init,补一次
        if (!initialized) init(context)
        return service
    }
}
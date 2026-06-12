package org.pastebridge.discovery

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import uniffi.paste_bridge_core.Discovery
import uniffi.paste_bridge_core.DiscoveryListener
import uniffi.paste_bridge_core.PasteBridgeException
import uniffiDiscovered = uniffi.paste_bridge_core.DiscoveredPeer

/**
 * Android-side [DiscoveryService] implementation.
 *
 * 直接委托给 UniFFI 生成的 Rust 绑定 (`uniffi.paste_bridge_core.Discovery`),
 * 与桌面端共用同一份 mDNS 注册/浏览逻辑,服务类型 `_pastebridge._tcp`、
 * TXT 字段 (`device_id` / `platform`) 全部对齐。
 *
 * 与之前 NsdManager 实现的差异:
 * - 不再使用 Android `NsdManager` 也不依赖 `NSD_SERVICE` 系统服务。
 * - 后台线程由 Rust 端 [Discovery] 持有,Kotlin 侧只需要实现
 *   [DiscoveryListener] 把回调转发到 [DiscoveredPeer] 流。
 *
 * 注: [DiscoveredPeer] (common 层) 与 `uniffi.paste_bridge_core.DiscoveredPeer`
 * (UniFFI 生成) 同名但属于不同包;回调桥接处做一次浅拷贝。
 */
actual class DiscoveryService actual constructor() {

    private val mainHandler = Handler(Looper.getMainLooper())

    @Volatile
    private var rustDiscovery: Discovery? = null

    @Volatile
    private var nativeListener: DiscoveryListener? = null

    @Volatile
    private var callbackBridge: CallbackBridge? = null

    /**
     * 保留 init 入口供 Android 端手动注入 Context (例如在 Activity 中)。
     * UniFFI 路径下不再需要 NsdManager,但保留函数签名以兼容现有调用方。
     */
    fun init(@Suppress("UNUSED_PARAMETER") context: Context) {
        Log.i(TAG, "init() called; DiscoveryService now backed by UniFFI Rust mDNS")
    }

    actual fun register(deviceId: String, platform: String, port: Int) {
        try {
            val discovery = ensureDiscovery()
            // 让 Rust 端自行查询本机所有 IPv4 接口地址;若以后需要更精细的
            // 地址过滤(例如排除虚拟网卡),可在 Rust 端加 [Discovery::register]
            // 的重载接受地址列表。
            val addresses: List<String> = emptyList()
            discovery.register(
                deviceId = deviceId,
                platform = platform,
                port = port.toUShort(),
                addresses = addresses,
            )
            Log.i(TAG, "register ok: deviceId=$deviceId platform=$platform port=$port")
        } catch (e: PasteBridgeException) {
            Log.e(TAG, "register failed: ${e.message}", e)
        }
    }

    actual fun browse(
        onDiscovered: (DiscoveredPeer) -> Unit,
        onLost: (DiscoveredPeer) -> Unit,
    ) {
        try {
            val discovery = ensureDiscovery()
            val bridge = CallbackBridge(
                mainHandler = mainHandler,
                onDiscovered = onDiscovered,
                onLost = onLost,
            )
            callbackBridge = bridge
            nativeListener = DiscoveryListenerProxy(bridge)
            discovery.browse(nativeListener!!)
            Log.i(TAG, "browse started")
        } catch (e: PasteBridgeException) {
            Log.e(TAG, "browse failed: ${e.message}", e)
        }
    }

    actual fun stop() {
        try {
            rustDiscovery?.let {
                // 触发 Rust 端 daemon shutdown;浏览线程收到 SearchStopped 后会退出。
                it.shutdown()
            }
        } catch (e: PasteBridgeException) {
            Log.w(TAG, "shutdown raised: ${e.message}")
        } finally {
            rustDiscovery = null
            nativeListener = null
            callbackBridge = null
        }
    }

    private fun ensureDiscovery(): Discovery {
        rustDiscovery?.let { return it }
        val d = Discovery()
        rustDiscovery = d
        return d
    }

    /**
     * 把 Rust 侧触发的回调桥接到 KMP 共享层 (currentThread -> mainThread),
     * 供 [DiscoveryService] 继续把事件推送到 `onDiscovered` / `onLost`。
     */
    private class CallbackBridge(
        private val mainHandler: Handler,
        private val onDiscovered: (DiscoveredPeer) -> Unit,
        private val onLost: (DiscoveredPeer) -> Unit,
    ) {
        fun dispatchDiscovered(peer: uniffiDiscovered) {
            val mapped = peer.toCommon()
            mainHandler.post { onDiscovered(mapped) }
        }

        fun dispatchLost(peer: uniffiDiscovered) {
            val mapped = peer.toCommon()
            mainHandler.post { onLost(mapped) }
        }
    }

    /**
     * UniFFI 生成的 [DiscoveryListener] 是 JNA callback,强引用被本类字段
     * ([nativeListener]) 持有,避免 callback 引用被 GC 释放。
     */
    private class DiscoveryListenerProxy(
        private val bridge: CallbackBridge,
    ) : DiscoveryListener {
        override fun onDiscovered(peer: uniffiDiscovered) {
            bridge.dispatchDiscovered(peer)
        }

        override fun onLost(peer: uniffiDiscovered) {
            bridge.dispatchLost(peer)
        }
    }

    /** UniFFI 生成的 [uniffiDiscovered] -> common 层 [DiscoveredPeer] 浅拷贝。 */
    private fun uniffiDiscovered.toCommon(): DiscoveredPeer = DiscoveredPeer(
        deviceId = deviceId,
        platform = platform,
        addresses = addresses.toList(),
        port = port.toInt(),
        fullname = fullname,
    )

    companion object {
        private const val TAG = "DiscoveryService"
    }
}

private fun Int.toUShort(): UShort = this.toUShort()


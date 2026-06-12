package org.pastebridge.sync

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import uniffi.paste_bridge_core.ApiServer
import uniffi.paste_bridge_core.ClipboardApiCallback
import uniffi.paste_bridge_core.DiscoveredPeer
import uniffi.paste_bridge_core.SyncReport

/**
 * Android 端 PasteBridge 同步服务。
 *
 * 职责：
 * 1. 启动 UniFFI [ApiServer]（callback 模式）监听 18792，让局域网内其它设备
 *    能 GET /clipboard/history 与 POST /clipboard/copy。
 * 2. 实现 [ClipboardApiCallback] 四个回调：
 *    - getHistoryJson(): 返回当前 in-memory 历史数组的 JSON 字节流
 *    - onRemoteCopy(text): 对端推送过来的文本，本地入历史 + 写 Android 系统剪贴板
 *    - 窗口可见性:移动端没有"窗口"概念,固定返回 false
 * 3. 监听 Android 系统剪贴板变化：用户从其它 app 复制时，自动入历史。
 * 4. 提供 [syncWithPeer] 包装:把 common 层 [DiscoveredPeer] 桥接到 UniFFI 绑定并执行。
 *
 * 历史存储: 暂用 in-memory ArrayDeque<HistoryEntry>,只保留最近 [MAX_HISTORY] 条。
 * 持久化可在后续接入 SQLite/DataStore(桌面端已经走 SQLite,Android 端先跑通闭环)。
 */
class SyncService private constructor(private val appContext: Context) {

    private val mainHandler = Handler(Looper.getMainLooper())
    private val clipboard = appContext.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager

    private val history: ArrayDeque<HistoryEntry> = ArrayDeque()
    private val historyLock = Any()
    private var windowVisible: Boolean = false

    @Volatile
    private var apiServer: ApiServer? = null

    @Volatile
    private var clipboardListener: ClipboardManager.OnPrimaryClipChangedListener? = null

    /**
     * 在 Activity.onCreate 里调用:启动 HTTP server + 监听系统剪贴板。
     * 多次调用幂等。
     */
    fun start(deviceId: String) {
        if (apiServer != null) {
            Log.i(TAG, "start() called but already running; ignoring")
            return
        }
        try {
            val server = ApiServer(PORT)
            server.startWithCallbacks(InMemoryCallback())
            apiServer = server
            Log.i(TAG, "ApiServer started on 0.0.0.0:, deviceId=")
        } catch (e: Throwable) {
            Log.e(TAG, "ApiServer start failed: ", e)
            return
        }
        // 监听 Android 系统剪贴板 -> 自动入历史
        val listener = ClipboardManager.OnPrimaryClipChangedListener { onSystemClipboardChanged() }
        clipboard.addPrimaryClipChangedListener(listener)
        clipboardListener = listener
        // 启动时把当前剪贴板内容也入历史(如果有)
        pushInitialClipboardIfAny()
    }

    fun stop() {
        clipboardListener?.let { clipboard.removePrimaryClipChangedListener(it) }
        clipboardListener = null
        apiServer = null
    }

    /**
     * 调 [uniffi.paste_bridge_core.syncWithPeer] 跑一次端到端同步。
     * 	extToPush 一般是本机最新一条文本（[latestText]）。
     * 同步是阻塞调用,建议在 IO 调度器里跑。
     */
    fun syncWithPeer(peer: DiscoveredPeer): SyncReport {
        val text = latestText().orEmpty()
        return uniffi.paste_bridge_core.syncWithPeer(peer, text)
    }

    fun latestText(): String? = synchronized(historyLock) {
        history.firstOrNull { it.contentType == "text" }?.text
    }

    fun historyJson(): ByteArray = synchronized(historyLock) {
        val arr = JSONArray()
        // history 是新的在前;UniFFI 端 get_history 默认也是 descending,保持一致
        for (e in history) {
            val obj = JSONObject()
            obj.put("id", e.id)
            obj.put("content_type", e.contentType)
            obj.put("content_text", e.text)
            obj.put("content_hash", e.contentHash)
            obj.put("mime_type", JSONObject.NULL)
            obj.put("file_size", JSONObject.NULL)
            obj.put("width", JSONObject.NULL)
            obj.put("height", JSONObject.NULL)
            obj.put("source_ip", JSONObject.NULL)
            obj.put("created_at", e.createdAt)
            obj.put("is_favorite", false)
            arr.put(obj)
        }
        arr.toString().toByteArray(Charsets.UTF_8)
    }

    private fun onSystemClipboardChanged() {
        try {
            val clip = clipboard.primaryClip ?: return
            if (clip.itemCount == 0) return
            val text = clip.getItemAt(0).coerceToText(appContext)?.toString() ?: return
            if (text.isEmpty()) return
            // 主线程回调;push 自身很快,直接调
            pushLocalText(text, source = "android-clipboard")
        } catch (e: Throwable) {
            Log.w(TAG, "onSystemClipboardChanged failed: ")
        }
    }

    private fun pushInitialClipboardIfAny() {
        try {
            val clip = clipboard.primaryClip ?: return
            if (clip.itemCount == 0) return
            val text = clip.getItemAt(0).coerceToText(appContext)?.toString() ?: return
            if (text.isEmpty()) return
            pushLocalText(text, source = "android-startup")
        } catch (e: Throwable) {
            // 无权限时 clipboard 抛 SecurityException,吞掉
        }
    }

    /**
     * 把文本塞进 in-memory 历史;同一 hash 视为重复,直接丢弃。
     */
    private fun pushLocalText(text: String, source: String) {
        val hash = sha1Short(text)
        synchronized(historyLock) {
            if (history.any { it.contentHash == hash }) {
                Log.d(TAG, "dup hash=, skip")
                return
            }
            val id = System.currentTimeMillis()
            val entry = HistoryEntry(
                id = id,
                contentType = "text",
                text = text,
                contentHash = hash,
                createdAt = id,
            )
            history.addFirst(entry)
            while (history.size > MAX_HISTORY) history.removeLast()
        }
        Log.i(TAG, "pushed local text (source=, len=, hash=)")
    }

    /**
     * 对端推送过来的文本: 入历史 + 写系统剪贴板(用户立即看到效果)。
     * 	ext 来自 [ClipboardApiCallback.onRemoteCopy],由 ApiServer 在 worker
     * 线程触发,需要切主线程访问剪贴板。
     */
    private fun onRemoteCopy(text: String) {
        mainHandler.post {
            try {
                pushLocalText(text, source = "remote")
                val clip = ClipData.newPlainText("pastebridge", text)
                clipboard.setPrimaryClip(clip)
                Log.i(TAG, "onRemoteCopy applied to system clipboard (len=)")
            } catch (e: Throwable) {
                Log.w(TAG, "onRemoteCopy failed: ")
            }
        }
    }

    private fun sha1Short(input: String): String {
        val md = java.security.MessageDigest.getInstance("SHA-1")
        val bytes = md.digest(input.toByteArray(Charsets.UTF_8))
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) sb.append(String.format("%02x", b))
        return sb.toString()
    }

    private inner class InMemoryCallback : ClipboardApiCallback {
        override fun getHistoryJson(): ByteArray = historyJson()

        override fun onRemoteCopy(text: String) {
            this@SyncService.onRemoteCopy(text)
        }

        override fun setWindowVisible(visible: Boolean) {
            windowVisible = visible
        }

        override fun isWindowVisible(): Boolean = windowVisible
    }

    private data class HistoryEntry(
        val id: Long,
        val contentType: String, // "text" | "image"
        val text: String,
        val contentHash: String,
        val createdAt: Long,
    )

    companion object {
        private const val TAG = "SyncService"
        const val PORT: UShort = 18792u
        private const val MAX_HISTORY = 200

        @Volatile
        private var INSTANCE: SyncService? = null

        fun get(context: Context): SyncService {
            return INSTANCE ?: synchronized(this) {
                INSTANCE ?: SyncService(context.applicationContext).also { INSTANCE = it }
            }
        }
    }
}

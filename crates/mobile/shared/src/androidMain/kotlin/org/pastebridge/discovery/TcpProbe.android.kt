package org.pastebridge.discovery

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import java.net.InetSocketAddress
import java.net.Socket

/**
 * 短超时 TCP connect。true 表示对端在监听,false 表示超时 / 拒绝 / 路由不可达。
 * 整个调用最多耗时 [timeoutMs] 毫秒。
 */
actual suspend fun tcpProbe(host: String, port: Int, timeoutMs: Int): Boolean =
    withContext(Dispatchers.IO) {
        withTimeoutOrNull(timeoutMs.toLong()) {
            try {
                Socket().use { sock ->
                    sock.connect(InetSocketAddress(host, port), timeoutMs)
                    true
                }
            } catch (e: Throwable) {
                false
            }
        } ?: false
    }

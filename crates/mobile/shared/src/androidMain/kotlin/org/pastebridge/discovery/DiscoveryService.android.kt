package org.pastebridge.discovery

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper
import android.util.Log

/**
 * Android 端 mDNS 实现：基于 [NsdManager] 的 `registerService` + `discoverServices`。
 *
 * 关键点：
 * - Android 的 NSD 不允许同一 service name 在同进程重复注册，因此 service name 加上端口后缀
 *   保证多次 register 不冲突（与 Rust 端 service_name 策略一致）
 * - browse 的回调在 NSD 内部线程触发，需要 `Handler(Looper.getMainLooper())` 切回主线程再分发给
 *   调用方，避免在 Compose UI 中触碰 state 时崩溃
 *
 * 使用方式：在 Activity.onCreate 中调用 [init] 注入 Context，再调用 [register] / [browse]。
 */
actual class DiscoveryService actual constructor() {

    private var nsdManager: NsdManager? = null

    private var registeredServiceName: String? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private val mainHandler = Handler(Looper.getMainLooper())

    /**
     * 注入 Android [Context] 并创建 [NsdManager]。必须在 [register] / [browse] 之前调用。
     * 多次调用以最后一次为准。
     */
    fun init(context: Context) {
        nsdManager =
            context.applicationContext.getSystemService(Context.NSD_SERVICE) as NsdManager
    }

    actual fun register(deviceId: String, platform: String, port: Int) {
        val manager = nsdManager ?: error("DiscoveryService.init(context) not called yet")
        if (registeredServiceName != null) {
            Log.w(TAG, "register called while already registered, skipping")
            return
        }
        val serviceName = "PasteBridge-${deviceId.take(12)}-$port"
        val info = NsdServiceInfo().apply {
            serviceName = serviceName
            serviceType = SERVICE_TYPE
            port = port
            txtRecord = mapOf(
                "device_id" to deviceId,
                "platform" to platform,
            )
        }
        manager.registerService(info, NsdManager.PROTOCOL_DNS_SD, registrationListener)
    }

    actual fun browse(onDiscovered: (DiscoveredPeer) -> Unit) {
        val manager = nsdManager ?: error("DiscoveryService.init(context) not called yet")
        if (discoveryListener != null) {
            Log.w(TAG, "browse called while already browsing, skipping")
            return
        }
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(regType: String) {
                Log.d(TAG, "onDiscoveryStarted: $regType")
            }

            override fun onDiscoveryStopped(serviceType: String) {
                Log.d(TAG, "onDiscoveryStopped: $serviceType")
            }

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "onStartDiscoveryFailed: $serviceType code=$errorCode")
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "onStopDiscoveryFailed: $serviceType code=$errorCode")
            }

            override fun onServiceFound(service: NsdServiceInfo) {
                Log.d(TAG, "onServiceFound: ${service.serviceName}")
                if (service.serviceType.contains(SERVICE_TYPE_WILDCARD) && service.serviceName != registeredServiceName) {
                    manager.resolveService(service, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(failedService: NsdServiceInfo, errorCode: Int) {
                            Log.e(TAG, "onResolveFailed: ${failedService.serviceName} code=$errorCode")
                        }

                        override fun onServiceResolved(resolved: NsdServiceInfo) {
                            val peer = resolved.toPeer()
                            Log.d(
                                TAG,
                                "onServiceResolved: ${peer.deviceId} @ ${peer.addresses}:${peer.port}",
                            )
                            mainHandler.post { onDiscovered(peer) }
                        }
                    })
                }
            }

            override fun onServiceLost(service: NsdServiceInfo) {
                Log.d(TAG, "onServiceLost: ${service.serviceName}")
            }
        }
        discoveryListener = listener
        manager.discoverServices(SERVICE_TYPE_WILDCARD, NsdManager.PROTOCOL_DNS_SD, listener)
    }

    actual fun stop() {
        nsdManager?.let { manager ->
            if (registeredServiceName != null) {
                try {
                    manager.unregisterService(registrationListener)
                } catch (e: Exception) {
                    Log.w(TAG, "unregisterService failed: ${e.message}")
                }
                registeredServiceName = null
            }
            discoveryListener?.let {
                try {
                    manager.stopServiceDiscovery(it)
                } catch (e: Exception) {
                    Log.w(TAG, "stopServiceDiscovery failed: ${e.message}")
                }
            }
        }
        discoveryListener = null
    }

    private val registrationListener = object : NsdManager.RegistrationListener {
        override fun onServiceRegistered(name: String) {
            Log.d(TAG, "onServiceRegistered: $name")
            registeredServiceName = name
        }

        override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
            Log.e(TAG, "onRegistrationFailed: ${serviceInfo.serviceName} code=$errorCode")
        }

        override fun onServiceUnregistered(arg0: NsdServiceInfo) {
            Log.d(TAG, "onServiceUnregistered: ${arg0.serviceName}")
            registeredServiceName = null
        }

        override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
            Log.e(TAG, "onUnregistrationFailed: ${serviceInfo.serviceName} code=$errorCode")
        }
    }

    private fun NsdServiceInfo.toPeer(): DiscoveredPeer {
        val attributes = attributes ?: emptyMap()
        val deviceId = attributes["device_id"]?.let { String(it) } ?: serviceName
        val platform = attributes["platform"]?.let { String(it) } ?: ""
        val addresses = host?.let { listOf(it.removePrefix("/")) } ?: emptyList()
        return DiscoveredPeer(
            deviceId = deviceId,
            platform = platform,
            addresses = addresses,
            port = port,
            fullname = "$serviceName.$serviceType",
        )
    }

    companion object {
        private const val TAG = "PBDiscovery"
        // Android NSD API 期望的服务类型是 `_pastebridge._tcp.` (末尾点号，不含 `local.`)
        private const val SERVICE_TYPE = "_pastebridge._tcp."
        private const val SERVICE_TYPE_WILDCARD = "_pastebridge._tcp."
    }
}
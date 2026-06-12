package org.pastebridge.app

import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import org.pastebridge.discovery.DiscoveryServiceProvider
import org.pastebridge.sync.SyncService

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        // 0. 注入 applicationContext 到 AppContext,供 expect/actual 拿 Context
        AppContext.applicationContext = applicationContext

        // 1. 注入 Context 给 DiscoveryService,后续 register/browse 即可用
        DiscoveryServiceProvider.init(applicationContext)
        // 2. 构造稳定 deviceId:用 ANDROID_ID (设备级,uninstall 后才会变),
        //    比硬编码 "android-smoke" 准,多设备时不会撞 device_id。
        val deviceId = Settings.Secure.getString(contentResolver, Settings.Secure.ANDROID_ID)
            ?: "android-"
        // 3. 启动 mDNS browse+register (仅 register,browse 交给 App() 内的 DiscoveryBanner)
        DiscoveryServiceProvider.tryGet(applicationContext).apply {
            register(
                deviceId = deviceId,
                platform = "android",
                port = SyncService.PORT.toInt(),
            )
        }
        // 4. 启动 ApiServer (callback 模式) + 系统剪贴板监听
        SyncService.get(applicationContext).start(deviceId)

        setContent {
            App()
        }
    }

    override fun onDestroy() {
        DiscoveryServiceProvider.tryGet(applicationContext).stop()
        SyncService.get(applicationContext).stop()
        super.onDestroy()
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    App()
}

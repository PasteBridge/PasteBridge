package org.pastebridge.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import org.pastebridge.discovery.DiscoveryServiceProvider

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        // 注入 Context 给 DiscoveryService,后续 browse/register 即可用
        DiscoveryServiceProvider.init(applicationContext)
        // 自动注册本机 + 浏览局域网其他设备 (设备 ID 可换成 SettingsView 的随机 UUID)
        DiscoveryServiceProvider.tryGet(applicationContext).apply {
            register(deviceId = "android-smoke", platform = "android", port = 18792)
            browse { peer ->
                android.util.Log.d(
                    "PBApp",
                    "discovered: ${peer.deviceId} ${peer.platform} @ ${peer.addresses}:${peer.port}",
                )
            }
        }

        setContent {
            App()
        }
    }

    override fun onDestroy() {
        DiscoveryServiceProvider.tryGet(applicationContext).stop()
        super.onDestroy()
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    App()
}
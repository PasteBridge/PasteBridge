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

        // 注入 Context 给 DiscoveryService,后续 register/browse 即可用
        DiscoveryServiceProvider.init(applicationContext)
        // 只在这里 register,browse 交给 App() 内的 DiscoveryBanner:
        // 让 Compose 的 peers state 跟着 mDNS 回调更新,Sheet 才能拿到数据
        DiscoveryServiceProvider.tryGet(applicationContext).apply {
            register(deviceId = "android-smoke", platform = "android", port = 18792)
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
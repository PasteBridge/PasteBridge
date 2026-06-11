package org.pastebridge.discovery

// iOS 暂未实现 TCP 探测(iOS 端整个发现也未实现),返回 false 让上层走
// "失败"分支,这样 UI 不会卡在"正在刷新"状态。
actual suspend fun tcpProbe(host: String, port: Int, timeoutMs: Int): Boolean = false

package org.pastebridge.discovery

/**
 * 主动 TCP 探测一个 host:port,判断设备是否真的活着。
 *
 * 为什么需要这个:mDNS/NSD 客户端在缓存过期前会"假阳性"地把已离线的设备
 * 当作在线(在新的 browse() 之后还会重新 fire onServiceFound)。仅靠 NSD
 * 自己的 TTL 要等几十秒到数分钟。
 *
 * 这里用一个短超时的 TCP connect 主动确认设备进程还在监听,通常 1 秒
 * 就能区分 alive / dead。
 *
 * 阻塞调用,建议在 IO 调度器里跑。
 */
expect suspend fun tcpProbe(host: String, port: Int, timeoutMs: Int = 1500): Boolean

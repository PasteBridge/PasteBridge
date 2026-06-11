package org.pastebridge.app

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import org.pastebridge.discovery.DiscoveredPeer
import org.pastebridge.discovery.DiscoveryService
import org.pastebridge.discovery.platformDiscovery

/**
 * 进程内 logcat 镜像:把 App/Discovery 关键事件缓冲到最近 N 条,UI 浮窗直接显示,
 * 方便开发期不用 adb 也能看到 NSD 注册 / 浏览回调是否触发。
 */
private object DebugLog {
    private const val MAX = 30
    private val lines = mutableListOf<String>()

    @Synchronized
    fun add(tag: String, msg: String) {
        val stamped = "${System.currentTimeMillis() % 100000} $tag: $msg"
        lines.add(stamped)
        if (lines.size > MAX) lines.removeAt(0)
    }

    @Synchronized
    fun snapshot(): List<String> = lines.toList()

    @Synchronized
    fun clear() = lines.clear()
}

private data class ClipboardItem(
    val id: Int,
    val content: String,
    /** 距离当前时间的分钟偏移量（如 5 = 5分钟前，120 = 2小时前） */
    val minuteOffset: Int,
    val source: String,
)

private fun relativeTimeLabel(minuteOffset: Int): String = when {
    minuteOffset < 1 -> "刚刚"
    minuteOffset < 60 -> "${minuteOffset}分钟前"
    minuteOffset < 1440 -> "${minuteOffset / 60}小时前"
    else -> "${minuteOffset / 1440}天前"
}

private val mockClipboardItems: List<ClipboardItem> by lazy {
    val sources = listOf("Chrome", "微信", "Notepad", "Terminal", "Android Studio", "Figma", "Slack", "Excel")
    val sampleLines = listOf(
        "项目启动会议纪要",
        "这是一段较长的剪贴板文本内容，用于测试多行显示效果。",
        "API密钥: sk-xxxx-xxxx-xxxx-xxxx",
        "https://example.com/docs/api-reference/overview",
        "待办事项：1) 完成登录模块 2) 修复滚动bug 3) 优化动画性能",
        "错误日志: NullPointerException at MainActivity.kt:42, caused by null user object in session manager initialization phase.",
    )
    val rng = kotlin.random.Random(seed = 42)
    List(100) { index ->
        val lineCount = rng.nextInt(1, 7)
        val content = buildString {
            repeat(lineCount) { lineIdx ->
                if (lineIdx > 0) appendLine()
                append(sampleLines[(index + lineIdx) % sampleLines.size])
            }
        }
        // minuteOffset 从 1 到 1440（24小时内）随机分布
        val minuteOffset = rng.nextInt(1, 1441)
        ClipboardItem(
            id = index,
            content = content,
            minuteOffset = minuteOffset,
            source = sources[index % sources.size],
        )
    }
}

private enum class Tab(val label: String, val icon: ImageVector) {
    Clipboard(label = "剪贴板", icon = IconsContentPaste),
    Debug(label = "调试", icon = IconsTerminal),
    Sync(label = "同步", icon = IconsSync),
    Settings(label = "设置", icon = IconsSettings),
}

@Composable
fun App() {
    // 提升发现状态到 App() 顶层,banner / FAB / Sheet 共用同一份数据
    val peers = remember { mutableStateOf<List<DiscoveredPeer>>(emptyList()) }
    val showDeviceList = remember { mutableStateOf(false) }
    // 浮窗 log 镜像(强制 Compose 订阅 snapshot 变化)
    val logVersion = remember { mutableStateOf(0) }
    DebugLog.add("App", "App() composed; debug overlay v${logVersion.value}")

    // 设备级"最近一次被看见"时间戳,用于把 mDNS TTL 2.5 分钟的丢失感知
    // 缩短到 ~30 秒:定时器周期重启 browse,被重启后还看不到的设备就视为丢失
    val lastSeen = remember { mutableStateOf<Map<String, Long>>(emptyMap()) }
    // 每次自增都会让 DiscoveryBanner 重新走 stop()+browse(),即"刷新一次发现"
    val refreshTick = remember { mutableStateOf(0) }
    // 手动刷新时正在被 TCP 验证的 peer fullname 集合:DiscoveryBanner
    // 收到这些 peer 的 onDiscovered 时必须忽略,只让 TCP 探测的结论
    // 决定它们是否回到列表(避免 NSD 缓存假阳性把验证中的 desktop 又加回)
    val verifyingFullnames = remember { mutableStateOf<Set<String>>(emptySet()) }
    // 用于手动刷新时的 TCP 探测协程
    val coroutineScope = rememberCoroutineScope()
    // 提升 DiscoveryService 到 App() 顶层,这样 onRefresh 可以在 Compose 重组
    // 发生之前**立即**调 stop()+browse(),避免 NSD 旧 listener 在 refreshTick++
    // 调度重组的空窗期里把缓存里的 desktop 又加回 peers。
    val discovery = platformDiscovery()

    // 后台定时器:每 15 秒触发一次刷新 + 清理超过 30 秒没被看见的设备
    androidx.compose.runtime.LaunchedEffect(Unit) {
        var tick = 0
        while (true) {
            kotlinx.coroutines.delay(15_000)
            tick += 1
            // 触发 DiscoveryBanner 走 DisposableEffect 重启 browse
            refreshTick.value = refreshTick.value + 1
            // 清理 30 秒以上没被刷到过的设备(应对 desktop 被 taskkill / 网络断开等
            // 不会发 RFC 6762 goodbye packet 的场景)
            val now = android.os.SystemClock.elapsedRealtime()
            val staleCutoff = 30_000L
            val before = peers.value.size
            val beforeMap = lastSeen.value
            val keepFullnames = beforeMap.filterValues { now - it <= staleCutoff }.keys
            val fresh = peers.value.filter { it.fullname in keepFullnames }
            if (fresh.size != before) {
                peers.value = fresh
                lastSeen.value = beforeMap.filterKeys { it in keepFullnames }
                DebugLog.add(
                    "PBApp",
                    "auto-prune: removed ${before - fresh.size} stale peer(s) at tick=$tick, total=${fresh.size}",
                )
                logVersion.value = logVersion.value + 1
            }
        }
    }

    PasteBridgeTheme {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
        ) {
            Column(modifier = Modifier.fillMaxSize()) {
                DiscoveryBanner(
                    peers = peers,
                    lastSeen = lastSeen,
                    verifyingFullnames = verifyingFullnames,
                    refreshTick = refreshTick.value,
                    onLog = { tag, msg ->
                        DebugLog.add(tag, msg)
                        logVersion.value = logVersion.value + 1
                    },
                )
                Box(modifier = Modifier.fillMaxSize()) {
                    ClipboardList(items = mockClipboardItems)
                    SyncFab(
                        count = peers.value.size,
                        onClick = { showDeviceList.value = true },
                        modifier = Modifier
                            .align(Alignment.BottomEnd)
                            .padding(end = 16.dp, bottom = 24.dp),
                    )
                    // 浮窗 debug 控制台:右上角,可折叠
                    DebugOverlay(
                        version = logVersion.value,
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(top = 56.dp, end = 8.dp),
                    )
                }
            }
            DeviceListSheet(
                peers = peers,
                visible = showDeviceList,
                onRefresh = {
                    // 手动刷新策略:
                    // 1. 立即 stop() 旧 listener,关闭 NSD 写入 peers 的通道
                    // 2. 立即清空 UI 列表(给用户"正在刷新"的反馈)
                    // 3. 用每个旧 peer 的 host:port 做一次短超时 TCP 探测
                    // 4. 探测成功的 peer 才放回列表(真正还活着的)
                    // 5. 探测失败的 peer 不会再次出现(已经被排除)
                    //
                    // 为什么需要 TCP 探测 + 立即 stop():Android NSD 在新 browse()
                    // 启动后,会对缓存里还没过期的服务重新 fire onServiceFound
                    // (resolveService 也会"成功",因为用的是本地缓存),这会让
                    // 已经关掉的 desktop 假阳性复活。仅靠 refreshTick++ 走
                    // Compose 重组,新 listener 还没装上时旧 listener 可能把
                    // 缓存里的 desktop 加回 peers。所以 onRefresh 第一步就
                    // 同步 stop() 旧 listener,关掉 NSD 写入通道。
                    val previousPeers = peers.value
                    try {
                        discovery?.stop()
                        DebugLog.add("PBApp", "manual refresh: stop() old listener (sync)")
                    } catch (e: Throwable) {
                        DebugLog.add("PBApp", "manual refresh: stop() failed: ${e.message}")
                    }
                    peers.value = emptyList()
                    lastSeen.value = emptyMap()
                    // 把"正在被 TCP 验证"的 peer 标记出来,DiscoveryBanner 收到这些
                    // peer 的 onServiceFound/onServiceLost 时直接忽略,避免 NSD
                    // 缓存的假阳性把它们又加回来
                    verifyingFullnames.value = previousPeers.map { it.fullname }.toSet()
                    refreshTick.value = refreshTick.value + 1
                    DebugLog.add("PBApp", "manual refresh: probing ${previousPeers.size} peer(s) via TCP")
                    logVersion.value = logVersion.value + 1
                    coroutineScope.launch {
                        for (peer in previousPeers) {
                            val host = peer.addresses.firstOrNull()
                            if (host.isNullOrBlank() || peer.port <= 0) {
                                verifyingFullnames.value = verifyingFullnames.value - peer.fullname
                                DebugLog.add("PBApp", "skip probe ${peer.deviceId.take(12)}: no addr/port")
                                continue
                            }
                            val alive = org.pastebridge.discovery.tcpProbe(host, peer.port, timeoutMs = 1200)
                            // 探测结束,从 verifying 集合里移除
                            verifyingFullnames.value = verifyingFullnames.value - peer.fullname
                            if (alive) {
                                DebugLog.add("PBApp", "tcp probe OK ${peer.deviceId.take(12)} @ $host:${peer.port}")
                                // 注意:append 到 peers 而不是覆盖,以免覆盖期间新发现
                                peers.value = (peers.value + peer).distinctBy { it.deviceId }
                                lastSeen.value = lastSeen.value + (peer.fullname to android.os.SystemClock.elapsedRealtime())
                            } else {
                                DebugLog.add("PBApp", "tcp probe FAIL ${peer.deviceId.take(12)} @ $host:${peer.port} -> dropped")
                            }
                            logVersion.value = logVersion.value + 1
                        }
                    }
                },
                onDismiss = { showDeviceList.value = false },
            )
        }
    }
}

/**
 * 顶部横幅：实时显示已发现的局域网 PasteBridge 设备。
 * - 无设备时显示「正在搜索…」
 * - 发现设备时列出 deviceId 与平台
 *
 * browse 回调在 Android 端已切到主线程,因此直接修改 mutableStateOf 安全。
 *
 * [refreshTick] 每次变化都会让 DisposableEffect 重新执行 —— 实际效果是
 * `discovery.stop()` 一次再 `browse()` 一次,把 NSD 缓存清空重发 query,
 * 把丢失感知从 mDNS TTL(2.5 分钟) 缩短到外层调度周期(15 秒)。
 */
@Composable
private fun DiscoveryBanner(
    peers: androidx.compose.runtime.MutableState<List<DiscoveredPeer>>,
    lastSeen: androidx.compose.runtime.MutableState<Map<String, Long>>,
    verifyingFullnames: androidx.compose.runtime.MutableState<Set<String>>,
    refreshTick: Int,
    onLog: (String, String) -> Unit,
) {
    val discovery = platformDiscovery()
    val scope = rememberCoroutineScope()
    androidx.compose.runtime.DisposableEffect(discovery, refreshTick) {
        if (discovery != null) {
            onLog("Banner", "DisposableEffect: starting browse (tick=$refreshTick) on ${discovery::class.simpleName}")
            // 先停上一次的 browse(如果是重启);Android 实现里 browse 不会在已有
            // discoveryListener 时再次启动,必须 stop() 后才能 browse()
            try {
                discovery.stop()
            } catch (e: Throwable) {
                onLog("Banner", "stop() failed (ignored): ${e.message}")
            }
            discovery.browse(
                onDiscovered = onDiscovered@{ peer ->
                    onLog("PBDiscovery", "found: ${peer.deviceId.take(12)} ${peer.platform} @ ${peer.addresses.firstOrNull() ?: "?"}:${peer.port}")
                    // 已经被手动刷新流程标记为"正在 TCP 验证"中的 peer,先忽略
                    // NSD 回调,等 TCP 探测结论再决定要不要放回列表。
                    if (peer.fullname in verifyingFullnames.value) {
                        onLog("PBApp", "skip NSD (tcp-verifying): ${peer.deviceId.take(12)}")
                        return@onDiscovered
                    }
                    // 已经在 peers 里的 peer,NSD 只是在重发,更新 lastSeen 即可;
                    // 但要**异步**做一次 TCP 探活:如果设备已死(没发 RFC 6762
                    // goodbye packet、桌面被 taskkill 等),NSD 仍可能在缓存里
                    // 持续重发,lastSeen 一直被刷新,30s 剪枝无法触达 —— 必须靠
                    // 主动 TCP 探测才能从 peers 里把它踢掉。
                    val knownIndex = peers.value.indexOfFirst { it.deviceId == peer.deviceId }
                    if (knownIndex >= 0) {
                        lastSeen.value = lastSeen.value + (peer.fullname to android.os.SystemClock.elapsedRealtime())
                        val knownPeer = peers.value[knownIndex]
                        val knownHost = knownPeer.addresses.firstOrNull()
                        if (!knownHost.isNullOrBlank() && knownPeer.port > 0) {
                            scope.launch {
                                val alive = org.pastebridge.discovery.tcpProbe(knownHost, knownPeer.port, timeoutMs = 800)
                                if (!alive) {
                                    onLog("PBApp", "drop (known tcp-dead): ${knownPeer.deviceId.take(12)} @ $knownHost:${knownPeer.port}")
                                    peers.value = peers.value.filterNot { it.deviceId == knownPeer.deviceId }
                                    lastSeen.value = lastSeen.value - knownPeer.fullname
                                }
                            }
                        }
                        return@onDiscovered
                    }
                    // 新发现的 peer:做一次 TCP 探测,确认它真的活着再加进 peers。
                    // 这是关键:Android NSD 缓存里已死的设备会在新 browse() 后重新
                    // 触发 onServiceFound,仅靠 lastSeen 30s 剪枝不够(因为 NSD 会
                    // 持续重发让 lastSeen 一直更新)。只有主动 TCP 探测能可靠区分
                    // 假阳性与真实存活。
                    val host = peer.addresses.firstOrNull()
                    if (host.isNullOrBlank() || peer.port <= 0) {
                        onLog("PBApp", "drop ${peer.deviceId.take(12)}: no addr/port")
                        return@onDiscovered
                    }
                    onLog("PBApp", "verify (tcp probe): ${peer.deviceId.take(12)} @ $host:${peer.port}")
                    scope.launch {
                        val alive = org.pastebridge.discovery.tcpProbe(host, peer.port, timeoutMs = 1000)
                        if (alive) {
                            // 探测成功才加入;用 distinctBy 防御 NSD 在极短时间内的
                            // 多次 onServiceFound 重复加入
                            peers.value = (peers.value + peer).distinctBy { it.deviceId }
                            lastSeen.value = lastSeen.value + (peer.fullname to android.os.SystemClock.elapsedRealtime())
                            onLog("PBApp", "discovered: ${peer.deviceId.take(12)} ${peer.platform} -> total=${peers.value.size}")
                        } else {
                            onLog("PBApp", "drop (tcp-dead): ${peer.deviceId.take(12)} @ $host:${peer.port}")
                        }
                    }
                },
                onLost = onLost@{ peer ->
                    onLog("PBDiscovery", "lost: fullname=${peer.fullname}")
                    // serviceName 用了 deviceId.take(12) + port,所以 lost 端拿不到完整 deviceId;
                    // 改用 fullname 匹配(两端都是从 "$serviceName.$serviceType" 拼出来的)
                    // 正在被 TCP 验证中的 peer 也不走 onLost(它的去留由探测决定)
                    if (peer.fullname in verifyingFullnames.value) {
                        onLog("PBApp", "skip onLost (tcp-verifying): ${peer.fullname}")
                        return@onLost
                    }
                    val before = peers.value.size
                    peers.value = peers.value.filterNot { it.fullname == peer.fullname }
                    lastSeen.value = lastSeen.value - peer.fullname
                    onLog("PBApp", "lost: ${peer.fullname} -> total=${peers.value.size} (was $before)")
                },
            )
        } else {
            onLog("Banner", "DisposableEffect: discovery is null (iOS?)")
        }
        onDispose {
            try {
                discovery?.stop()
            } catch (e: Throwable) {
                onLog("Banner", "dispose stop() failed (ignored): ${e.message}")
            }
        }
    }

    val bg = if (peers.value.isEmpty()) {
        MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
    } else {
        MaterialTheme.colorScheme.primaryContainer
    }
    val text = if (peers.value.isEmpty()) {
        "🔍 正在搜索局域网设备…"
    } else {
        "📡 已发现 ${peers.value.size} 台设备: " +
            peers.value.joinToString(" · ") { "${it.deviceId.take(8)} (${it.platform})" }
    }
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .background(bg)
            .padding(horizontal = 12.dp, vertical = 6.dp),
    ) {
        Text(text = text, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurface)
    }
}

/**
 * 浮窗 debug 控制台:右上角显示最近 N 条 log,点击展开/收起。
 * 解决"看不到 device list 是否真填充"的问题——不用 adb 也能看到 browse 事件。
 */
@Composable
private fun DebugOverlay(version: Int, modifier: Modifier = Modifier) {
    var expanded by remember { mutableStateOf(false) }
    // version 改变时强制重新读 snapshot
    val lines = remember(version) { DebugLog.snapshot() }

    Column(
        modifier = modifier
            .background(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.92f),
                shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp),
            )
            .border(
                width = 1.dp,
                color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f),
                shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp),
            )
            .padding(8.dp)
            .widthIn(max = 280.dp),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.fillMaxWidth().clickable { expanded = !expanded },
        ) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .background(MaterialTheme.colorScheme.primary, shape = CircleShape),
            )
            Spacer(Modifier.width(6.dp))
            Text(
                text = "debug (${lines.size})",
                fontSize = 10.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.weight(1f))
            Text(
                text = if (expanded) "▾" else "▸",
                fontSize = 10.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (expanded) {
            Spacer(Modifier.height(4.dp))
            androidx.compose.foundation.lazy.LazyColumn(
                modifier = Modifier.heightIn(max = 220.dp),
            ) {
                items(lines.size) { i ->
                    Text(
                        text = lines[i],
                        fontSize = 9.sp,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.padding(vertical = 1.dp),
                    )
                }
            }
        }
    }
}

/**
 * 右下角悬浮的同步按钮:点开显示设备列表。
 * 设备数为 0 时显示「同步」,>0 时徽章显示数量。
 */
@Composable
private fun SyncFab(
    count: Int,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    androidx.compose.material3.ExtendedFloatingActionButton(
        onClick = onClick,
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.primaryContainer,
        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        icon = { Icon(imageVector = IconsSync, contentDescription = null) },
        text = {
            Text(
                text = if (count > 0) "同步 ($count)" else "同步",
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
            )
        },
    )
}

/**
 * 底部弹层:展示已发现的 PasteBridge 设备列表,每行可点击触发后续同步动作。
 * 状态: empty / list
 */
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun DeviceListSheet(
    peers: androidx.compose.runtime.MutableState<List<DiscoveredPeer>>,
    visible: androidx.compose.runtime.MutableState<Boolean>,
    onRefresh: () -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = androidx.compose.material3.rememberModalBottomSheetState(skipPartiallyExpanded = true)
    if (visible.value) {
        androidx.compose.material3.ModalBottomSheet(
            onDismissRequest = onDismiss,
            sheetState = sheetState,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 8.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "附近的设备",
                            fontSize = 18.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = "通过 mDNS 在局域网内自动发现",
                            fontSize = 12.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 2.dp),
                        )
                    }
                    androidx.compose.material3.IconButton(onClick = onRefresh) {
                        Text(
                            text = "↻",
                            fontSize = 22.sp,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
                Spacer(Modifier.height(12.dp))

                if (peers.value.isEmpty()) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 32.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = "暂未发现其他设备\n请确认两端在同一 Wi-Fi 局域网",
                            fontSize = 13.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                        )
                    }
                } else {
                    androidx.compose.foundation.lazy.LazyColumn(
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        items(
                            items = peers.value,
                            key = { it.deviceId },
                        ) { peer ->
                            DeviceRow(peer = peer, onClick = {
                                // 后续接: 推送本机最新剪贴板 / 拉取对端历史
                                onDismiss()
                            })
                            HorizontalDivider(
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.1f),
                                thickness = 1.dp,
                            )
                        }
                    }
                }

                Spacer(Modifier.height(16.dp))
            }
        }
    }
}

@Composable
private fun DeviceRow(peer: DiscoveredPeer, onClick: () -> Unit) {
    val platformBadgeColor = when (peer.platform) {
        "android" -> Color(0xFF3DDC84)
        "ios" -> Color(0xFF007AFF)
        "desktop" -> Color(0xFFFF9800)
        else -> Color(0xFF9E9E9E)
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(vertical = 12.dp, horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(10.dp)
                .background(platformBadgeColor, shape = CircleShape),
        )
        Spacer(Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = peer.deviceId.take(16),
                fontSize = 15.sp,
                fontWeight = FontWeight.Medium,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Text(
                text = "${peer.platform} · ${peer.addresses.firstOrNull() ?: "—"}:${peer.port}",
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
        Icon(
            imageVector = IconsSync,
            contentDescription = "同步",
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.size(20.dp),
        )
    }
}

@Composable
private fun PasteBridgeTheme(
    useDarkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme: ColorScheme = if (useDarkTheme) {
        darkColorScheme(
            primary = Color(0xFFA8C7FF),
            secondary = Color(0xFFC2C7DD),
            tertiary = Color(0xFFE0B5FF),
            background = Color(0xFF101418),
            surface = Color(0xFF101418),
            primaryContainer = Color(0xFF1E3A66),
            onPrimaryContainer = Color(0xFFD6E3FF),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF2A5DB0),
            secondary = Color(0xFF585E72),
            tertiary = Color(0xFF765491),
            background = Color(0xFFF8F9FF),
            surface = Color(0xFFF8F9FF),
            primaryContainer = Color(0xFFD6E3FF),
            onPrimaryContainer = Color(0xFF001B3E),
        )
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content,
    )
}

@Composable
private fun TabBody(title: String, subtitle: String) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .safeContentPadding()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .size(72.dp)
                .background(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = CircleShape,
                ),
        )
        Text(
            text = title,
            fontSize = 28.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(top = 24.dp),
        )
        Text(
            text = subtitle,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 8.dp),
        )
    }
}

@Composable
private fun DebugPanel() {
    val currentTime = remember { java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss", java.util.Locale.getDefault()).format(java.util.Date()) }
    var refreshCount by remember { mutableStateOf(0) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .safeContentPadding()
            .padding(16.dp),
    ) {
        // 标题
        Text(
            text = "调试信息",
            fontSize = 24.sp,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        
        Spacer(Modifier.height(16.dp))
        
        // 调试日志区域
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f)
                .background(
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp),
                )
                .padding(12.dp),
        ) {
            item {
                DebugLogItem("时间", currentTime)
                DebugLogItem("应用版本", "1.0.0")
                DebugLogItem("构建类型", "Debug")
                DebugLogItem("平台", "Android/iOS")
                DebugLogItem("Compose版本", "1.11.0")
            }
        }
        
        Spacer(Modifier.height(16.dp))
        
        // 刷新按钮
        androidx.compose.material3.Button(
            onClick = { refreshCount++ },
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("刷新调试信息")
        }
        
        Spacer(Modifier.height(8.dp))
        
        Text(
            text = "刷新次数: $refreshCount",
            fontSize = 12.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun DebugLogItem(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            fontSize = 14.sp,
            fontWeight = FontWeight.Medium,
            color = MaterialTheme.colorScheme.primary,
        )
        Text(
            text = value,
            fontSize = 14.sp,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}

@Composable
private fun ClipboardList(items: List<ClipboardItem>) {
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val showScrollToTop by remember {
        derivedStateOf { listState.firstVisibleItemIndex > 3 || listState.firstVisibleItemScrollOffset > 200 }
    }
    val firstVisibleLabel by remember {
        derivedStateOf {
            val index = listState.firstVisibleItemIndex
            if (index < items.size) relativeTimeLabel(items[index].minuteOffset) else ""
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(start = 12.dp, end = 12.dp, top = 24.dp),
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
        ) {
            // 顶部内容区域
            Row(
                modifier = Modifier.weight(1f),
            ) {
                // 左侧列表
                LazyColumn(
                    state = listState,
                    modifier = Modifier.weight(1f),
                    contentPadding = PaddingValues(vertical = 0.dp),
                ) {
                    stickyHeader(key = "time_header") {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .background(MaterialTheme.colorScheme.background)
                                .padding(horizontal = 4.dp, vertical = 4.dp),
                        ) {
                            // 时间标签
                            Text(
                                text = firstVisibleLabel,
                                fontSize = 12.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(start = 4.dp, top = 2.dp, bottom = 2.dp),
                            )
                        }
                    }

                    items(
                        items = items,
                        key = { it.id },
                    ) { item ->
                        Column(modifier = Modifier.fillMaxWidth()) {
                            ClipboardItemCard(
                                item = item,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 2.dp, vertical = 4.dp),
                            )
                            HorizontalDivider(
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.1f),
                                thickness = 1.dp,
                                modifier = Modifier.padding(horizontal = 2.dp),
                            )
                        }
                    }
                }

                // 右侧工具栏
                Column(
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .fillMaxHeight(),
                    verticalArrangement = Arrangement.Top,
                ) {
                    IconButton(
                        onClick = { /* mock: 添加 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsAdd,
                            contentDescription = "添加",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 置顶 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsPushPin,
                            contentDescription = "置顶",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 分享 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsShare,
                            contentDescription = "分享",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 删除 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsDelete,
                            contentDescription = "删除",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 收藏 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsFavorite,
                            contentDescription = "收藏",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 筛选 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsFilterList,
                            contentDescription = "筛选",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 排序 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsSort,
                            contentDescription = "排序",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 新到旧 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsArrowDownward,
                            contentDescription = "新到旧",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(
                        onClick = { /* mock: 旧到新 */ },
                        modifier = Modifier.size(40.dp),
                    ) {
                        Icon(
                            imageVector = IconsArrowUpward,
                            contentDescription = "旧到新",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // 底部搜索栏
            SearchBar()
        }

        if (showScrollToTop) {
            FloatingActionButton(
                onClick = { scope.launch { listState.animateScrollToItem(index = 0) } },
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 60.dp, bottom = 70.dp)
                    .size(48.dp),
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary,
            ) {
                Icon(
                    imageVector = IconsKeyboardArrowUp,
                    contentDescription = "返回顶部",
                    modifier = Modifier.size(24.dp),
                )
            }
        }
    }
}

@Composable
private fun SearchBar() {
    var searchText by remember { mutableStateOf("") }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        androidx.compose.material3.OutlinedTextField(
            value = searchText,
            onValueChange = { searchText = it },
            modifier = Modifier.weight(1f),
            placeholder = {
                Text(
                    text = "搜索剪贴板内容...",
                    fontSize = 14.sp,
                )
            },
            singleLine = true,
        )
        Spacer(Modifier.width(8.dp))
        IconButton(
            onClick = { /* mock: 搜索 */ },
            modifier = Modifier.size(40.dp),
        ) {
            Icon(
                imageVector = IconsFilterList,
                contentDescription = "搜索",
                modifier = Modifier.size(20.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

@Composable
private fun ClipboardItemCard(
    item: ClipboardItem,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.Top,
    ) {
        // 左侧竖线分隔符
        Box(
            modifier = Modifier
                .padding(top = 4.dp, bottom = 4.dp)
                .width(2.dp)
                .height(24.dp)
                .background(
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f),
                ),
        )
        Spacer(Modifier.width(8.dp))
        Column(modifier = Modifier.fillMaxWidth()) {
            // 来源行
            Text(
                text = item.source,
                fontSize = 15.sp,
                fontWeight = FontWeight.Medium,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.padding(top = 2.dp, bottom = 2.dp),
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = item.content,
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onBackground,
                maxLines = 6,
                overflow = TextOverflow.Ellipsis,
                lineHeight = 24.sp,
            )
        }
    }
}



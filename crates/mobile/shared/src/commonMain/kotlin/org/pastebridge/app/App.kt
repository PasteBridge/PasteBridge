package org.pastebridge.app

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
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
    PasteBridgeTheme {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
        ) {
            Column(modifier = Modifier.fillMaxSize()) {
                DiscoveryBanner()
                ClipboardList(items = mockClipboardItems)
            }
        }
    }
}

/**
 * 顶部横幅：实时显示已发现的局域网 PasteBridge 设备。
 * - 无设备时显示「正在搜索…」
 * - 发现设备时列出 deviceId 与平台
 *
 * browse 回调在 Android 端已切到主线程,因此直接修改 mutableStateOf 安全。
 */
@Composable
private fun DiscoveryBanner() {
    val discovery = platformDiscovery()
    val peers by remember(discovery) {
        androidx.compose.runtime.mutableStateOf<List<DiscoveredPeer>>(emptyList())
    }
    androidx.compose.runtime.DisposableEffect(discovery) {
        if (discovery != null) {
            discovery.browse { peer ->
                peers.value = (peers.value + peer).distinctBy { it.deviceId }
            }
        }
        onDispose { /* DiscoveryService.stop() 由 Activity onDestroy 调用 */ }
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



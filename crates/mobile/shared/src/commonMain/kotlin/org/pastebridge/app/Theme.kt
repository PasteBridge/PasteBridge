package org.pastebridge.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable

/**
 * 各平台暗色主题检测。
 * 在 Android 上会检查系统的 Configuration.uiMode；
 * 在 iOS 上会查询系统 UITraitCollection.userInterfaceStyle。
 *
 * 注意：本工程当前主要关注 Android，iOS 实现作为占位返回 false。
 */
@Composable
@ReadOnlyComposable
expect fun isSystemInDarkTheme(): Boolean

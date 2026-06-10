package org.pastebridge.app

import androidx.compose.ui.graphics.vector.ImageVector

/**
 * 图标资源桥接层。
 * commonMain 声明 expect，androidMain 提供 actual 实现。
 * 图标来源：androidx.compose.material:material-icons-extended（仅 Android）。
 */
expect val IconsContentPaste: ImageVector
expect val IconsTerminal: ImageVector
expect val IconsSync: ImageVector
expect val IconsSettings: ImageVector
expect val IconsAdd: ImageVector
expect val IconsPushPin: ImageVector
expect val IconsShare: ImageVector
expect val IconsDelete: ImageVector
expect val IconsFavorite: ImageVector
expect val IconsFilterList: ImageVector
expect val IconsSort: ImageVector
expect val IconsArrowDownward: ImageVector
expect val IconsArrowUpward: ImageVector
expect val IconsKeyboardArrowUp: ImageVector

package org.pastebridge.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import platform.UIKit.UIUserInterfaceStyle
import platform.UIKit.UIScreen

@Composable
@ReadOnlyComposable
actual fun isSystemInDarkTheme(): Boolean =
    UIScreen.mainScreen.traitCollection.userInterfaceStyle ==
        UIUserInterfaceStyle.UIUserInterfaceStyleDark

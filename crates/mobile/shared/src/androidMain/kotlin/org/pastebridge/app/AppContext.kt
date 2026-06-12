package org.pastebridge.app

import android.content.Context

/**
 * 持有 ApplicationContext 的轻量容器。
 * 由 [MainActivity.onCreate] 注入,供 common 层（特别是 expect/actual 实现）
 * 在没有 Activity 引用时也能拿到 Context,例如系统剪贴板访问、ApiServer 启动等。
 *
 * 注意:这只存了 applicationContext,不要在这里缓存 Activity / View,避免泄漏。
 */
object AppContext {
    @Volatile
    var applicationContext: Context? = null
}

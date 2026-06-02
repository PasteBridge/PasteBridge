package org.pastebridge.app

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform
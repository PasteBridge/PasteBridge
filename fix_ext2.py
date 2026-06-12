# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/org/pastebridge/discovery/DiscoveryService.android.kt')
text = p.read_text(encoding='utf-8')
old = '    /** UniFFI 生成的 [uniffiDiscovered] -> common 层 [DiscoveredPeer] 浅拷贝。 */\n    private fun uniffi.paste_bridge_core.DiscoveredPeer.toCommon(): DiscoveredPeer = DiscoveredPeer(\n        deviceId = deviceId,\n        platform = platform,\n        addresses = addresses.toList(),\n        port = port.toInt(),\n        fullname = fullname,\n    )'
new = '    /** UniFFI 生成的 [uniffi.paste_bridge_core.DiscoveredPeer] -> common 层 [DiscoveredPeer] 浅拷贝。 */\n    private fun uniffiDiscoveredToCommon(peer: uniffi.paste_bridge_core.DiscoveredPeer): DiscoveredPeer = DiscoveredPeer(\n        deviceId = peer.deviceId,\n        platform = peer.platform,\n        addresses = peer.addresses.toList(),\n        port = peer.port.toInt(),\n        fullname = peer.fullname,\n    )'
assert old in text, 'old block not found'
text = text.replace(old, new)
text = text.replace('val mapped = peer.toCommon()', 'val mapped = uniffiDiscoveredToCommon(peer)')
p.write_text(text, encoding='utf-8')
print('patched')

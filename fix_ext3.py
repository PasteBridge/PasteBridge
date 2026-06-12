# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/org/pastebridge/discovery/DiscoveryService.android.kt')
text = p.read_text(encoding='utf-8')
# Move the helper to file level: remove from inside the class, add at end of file
inner = '    /** UniFFI 生成的 [uniffi.paste_bridge_core.DiscoveredPeer] -> common 层 [DiscoveredPeer] 浅拷贝。 */\n    private fun uniffiDiscoveredToCommon(peer: uniffi.paste_bridge_core.DiscoveredPeer): DiscoveredPeer = DiscoveredPeer(\n        deviceId = peer.deviceId,\n        platform = peer.platform,\n        addresses = peer.addresses.toList(),\n        port = peer.port.toInt(),\n        fullname = peer.fullname,\n    )\n'
file_level = '\n/** UniFFI 生成的 [uniffi.paste_bridge_core.DiscoveredPeer] -> common 层 [DiscoveredPeer] 浅拷贝。 */\nprivate fun uniffiDiscoveredToCommon(peer: uniffi.paste_bridge_core.DiscoveredPeer): DiscoveredPeer = DiscoveredPeer(\n    deviceId = peer.deviceId,\n    platform = peer.platform,\n    addresses = peer.addresses.toList(),\n    port = peer.port.toInt(),\n    fullname = peer.fullname,\n)\n'
assert inner in text, 'inner not found'
text = text.replace(inner, '')
# Append to end (after the last private fun Int.toUShort())
text = text.rstrip() + '\n' + file_level
p.write_text(text, encoding='utf-8')
print('patched')

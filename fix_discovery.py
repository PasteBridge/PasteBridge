# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/org/pastebridge/discovery/DiscoveryService.android.kt')
text = p.read_text(encoding='utf-8')
# Replace the alias import with full qualifier usage
text = text.replace('import uniffiDiscovered = uniffi.paste_bridge_core.DiscoveredPeer', '')
text = text.replace('peer: uniffiDiscovered', 'peer: uniffi.paste_bridge_core.DiscoveredPeer')
text = text.replace('private fun uniffiDiscovered.toCommon(): DiscoveredPeer', 'private fun uniffi.paste_bridge_core.DiscoveredPeer.toCommon(): DiscoveredPeer')
# Use property access on the FQN type - this should still work with Kotlin's data class member access
p.write_text(text, encoding='utf-8')
print('patched')
print(text[:2000])

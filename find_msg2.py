# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Look around position 83744
chunk = text[83730:83770]
print(repr(chunk))
print('bytes:', [hex(b) for b in chunk.encode('utf-8')])

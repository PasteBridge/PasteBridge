# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
BT = chr(0x60)
old1 = 'value' + BT + 'message' + BT
new1 = 'value.errorMessage'
print('count before:', text.count(old1))
text = text.replace(old1, new1)
print('count after:', text.count(old1))
p.write_text(text, encoding='utf-8')
print('patched')

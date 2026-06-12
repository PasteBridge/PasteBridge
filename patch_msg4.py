# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
old = 'value' + chr(0x60) + 'message' + chr(0x60)
new = 'value.errorMessage'
print('searching for:', repr(old))
print('count:', text.count(old))
print('first match idx:', text.find(old))

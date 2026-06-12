# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Test exact substring
test = 'value' + chr(0x60) + 'message' + chr(0x60)
print('test in text:', test in text)
# Try alternative encoding
test2 = 'value' + '\u0060' + 'message' + '\u0060'
print('test2 in text:', test2 in text)
# Look at character at 83744
ch = text[83744]
print('char at 83744:', repr(ch), 'code:', ord(ch))
# Look at 83745
print('char at 83745:', repr(text[83745]), 'code:', ord(text[83745]))

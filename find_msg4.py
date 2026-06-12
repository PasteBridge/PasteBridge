# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Get chunk of text containing the backticks
chunk = text[83740:83760]
print('chunk:', repr(chunk))
# Try to find the literal chunk
target = chunk[5:18]  # e.message part
print('target:', repr(target))
print('target in text:', target in text)
# Let's check encoding
print('chunk encoded:', chunk.encode('utf-8').hex())
# Now try with our test
test = 'value' + chr(0x60) + 'message' + chr(0x60)
print('test:', repr(test))
print('test encoded:', test.encode('utf-8').hex())

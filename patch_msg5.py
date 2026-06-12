# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
old = 'value.' + chr(0x60) + 'message' + chr(0x60)
new = 'value.errorMessage'
print('count before:', text.count(old))
text = text.replace(old, new)
print('count after:', text.count(old))
p.write_text(text, encoding='utf-8')
# Verify
lines = text.splitlines(keepends=False)
for i in range(2113, 2128):
    print(i+1, repr(lines[i]))

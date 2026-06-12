# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
text = text.replace('}ion object ErrorHandler', '}\n    companion object ErrorHandler')
p.write_text(text, encoding='utf-8')
print('fixed')
import sys
lines = text.splitlines(keepends=False)
for i in range(2086, 2095):
    print(i+1, repr(lines[i]))

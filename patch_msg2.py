# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Fix the corrupted line 2089 - it lost 'companion o' prefix
text = text.replace(
    '    }bject ErrorHandler',
    '    companion object ErrorHandler'
)
# Fix the converter uses of message (with backticks) to errorMessage
text = text.replace('value.message', 'value.errorMessage')
p.write_text(text, encoding='utf-8')
# Verify
lines = text.splitlines(keepends=False)
for i in range(2085, 2095):
    print(i+1, repr(lines[i]))
print('---')
for i in range(2113, 2128):
    print(i+1, repr(lines[i]))

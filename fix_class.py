# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Replace the class block
old = '    class Generic(\n        \n        val errorMessage: kotlin.String\n    ) : PasteBridgeException(errorMessage) {\n        // 跳过 override val message 避免与字段名同名冲突\n        val errorMessageAsString: kotlin.String get() = "errorMessage=" + errorMessage\n    }'
new = '    class Generic(\n        val errorMessage: kotlin.String\n    ) : PasteBridgeException() {\n        // 跳过 override val message 避免与字段名同名冲突\n        val errorMessageAsString: kotlin.String get() = "errorMessage=" + errorMessage\n    }'
assert old in text, 'block not found'
text = text.replace(old, new)
p.write_text(text, encoding='utf-8')
print('patched')
# Verify
lines = text.splitlines(keepends=False)
for i in range(2082, 2095):
    print(i+1, repr(lines[i]))

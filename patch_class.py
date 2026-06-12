# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
lines = p.read_text(encoding='utf-8').splitlines(keepends=False)
lines[2085] = '    ) : PasteBridgeException() {'
lines[2086] = '        // 跳过 override val message 避免与字段名同名冲突'
lines[2087] = '        val \u0060messageAsString\u0060: kotlin.String get() = "message=" + \u0060message\u0060'
lines[2088] = '    }'
p.write_text('\n'.join(lines) + '\n', encoding='utf-8')
print('patched')
for i in range(2082, 2095):
    print(i+1, repr(lines[i]))

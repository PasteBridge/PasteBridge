# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# Find class Generic and patch
i = text.find('class Generic(')
old = text[i:i+250]
print('OLD:')
print(repr(old))
new = 'class Generic(\n        \n        val errorMessage: kotlin.String\n    ) : PasteBridgeException(errorMessage) {\n        // 跳过 override val message 避免与字段名同名冲突\n        val errorMessageAsString: kotlin.String get() = "errorMessage=" + errorMessage\n    }'
text = text[:i] + new + text[i+250:]
text = text.replace('FfiConverterString.allocationSize(value.message)',
                    'FfiConverterString.allocationSize(value.errorMessage)')
text = text.replace('FfiConverterString.write(value.message, buf)',
                    'FfiConverterString.write(value.errorMessage, buf)')
p.write_text(text, encoding='utf-8')
print('patched')
# Verify
i2 = text.find('class Generic(')
print('NEW:')
print(text[i2:i2+250])

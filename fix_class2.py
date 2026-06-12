# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
i = text.find('class Generic(')
old_block = text[i:i+252]
new_block = 'class Generic(\n        \n        val \u0060errorMessage\u0060: kotlin.String\n    ) : PasteBridgeException() {\n        // 跳过 override val message 避免与字段名同名冲突\n        val \u0060errorMessageAsString\u0060: kotlin.String get() = "errorMessage=" + \u0060errorMessage\u0060\n    }'
text = text[:i] + new_block + text[i+252:]
p.write_text(text, encoding='utf-8')
print('patched')
# Also need to update converter to use backticks
text2 = p.read_text(encoding='utf-8')
# The converter uses value.errorMessage (no backticks) - need to keep that
print('converter still uses value.errorMessage:', 'value.errorMessage' in text2)

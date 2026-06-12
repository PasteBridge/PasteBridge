# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt')
text = p.read_text(encoding='utf-8')
old = '        super.onCreate(savedInstanceState)'
new = '        enableEdgeToEdge()\n        super.onCreate(savedInstanceState)'
text = text.replace(old, new)
p.write_text(text, encoding='utf-8')
print('patched')

# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt')
text = p.read_text(encoding='utf-8')
# Add System.setProperty for jna.nosys before any JNA usage
old = '        enableEdgeToEdge()\n        super.onCreate(savedInstanceState)'
new = '        // 告诉 JNA 不要从 classpath 加载 libjnidispatch.so,改用 System.loadLibrary(\"jnidispatch\")\n        // libjnidispatch.so 已经在 APK 的 lib/x86 和 lib/x86_64 中\n        System.setProperty(\"jna.nosys\", \"true\")\n        enableEdgeToEdge()\n        super.onCreate(savedInstanceState)'
text = text.replace(old, new)
p.write_text(text, encoding='utf-8')
print('patched')
print(text)

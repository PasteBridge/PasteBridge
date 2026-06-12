# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt')
text = p.read_text(encoding='utf-8')
# Move the System.setProperty to a companion object init block
old = 'class MainActivity : ComponentActivity() {\n    override fun onCreate(savedInstanceState: Bundle?) {\n        // 告诉 JNA 不要从 classpath 加载 libjnidispatch.so,改用 System.loadLibrary(\"jnidispatch\")\n        // libjnidispatch.so 已经在 APK 的 lib/x86 和 lib/x86_64 中\n        System.setProperty(\"jna.nosys\", \"true\")\n        enableEdgeToEdge()'
new = 'class MainActivity : ComponentActivity() {\n    companion object {\n        init {\n            // 告诉 JNA 使用 System.loadLibrary 加载 libjnidispatch.so 而不是从 classpath 资源加载\n            // libjnidispatch.so 已经在 APK 的 lib/x86 和 lib/x86_64 中\n            System.setProperty(\"jna.nosys\", \"true\")\n        }\n    }\n    override fun onCreate(savedInstanceState: Bundle?) {'
if old not in text:
    print('old not found! showing context:')
    i = text.find('onCreate')
    print(text[i:i+400])
    exit(1)
text = text.replace(old, new)
p.write_text(text, encoding='utf-8')
print('patched')
# verify
for i, line in enumerate(p.read_text(encoding='utf-8').splitlines(), start=1):
    if 'jna' in line or 'companion' in line or 'onCreate' in line:
        print(f'{i}: {line}')

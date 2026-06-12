# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='utf-8')
text = text.replace(
    '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            implementation("net.java.dev.jna:jna:5.13.0")',
    '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            // \u81ea\u5b9a\u4e49 JNA jar \u5185\u7f6e\u4e86 android-x86/x86_64/arm/arm64 \u5e73\u53f0\u7684 libjnidispatch.so\n            implementation(files("\''\\''/../../target/jna-5.13.0-android.jar"))'
)
p.write_text(text, encoding='utf-8')
print('patched')
import sys
for line in text.splitlines():
    if 'jna' in line.lower() or 'UniFFI' in line:
        print(line)

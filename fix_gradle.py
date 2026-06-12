# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Target: fix the mess at lines 35-41 (1-indexed) where:
#   35: '            // UniFFI 在 Android 端使用 JNA 与 Rust 通信'
#   36: '            // 自定义 JNA jar 内置了 ...'
#   37: '            implementation(files("/../../target/jna-5.13.0-android.jar"))'
#   38: '                // 留作占位,实际由 repositories 中的 flatDir 替换'
#   39: '            }'
#   40: '            implementation(libs.compose.material.icons.extended)'
# Goal: replace lines 35-39 with the correct single implementation line.
target_text = '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            // \u81ea\u5b9a\u4e49 JNA jar \u5185\u7f6e\u4e86 android-x86/x86_64/arm/arm64 \u5e73\u53f0\u7684 libjnidispatch.so\n            implementation(files("\''\\''/../../target/jna-5.13.0-android.jar"))'
print('target found:', target_text in text)
# Find the broken block by searching for the placeholder
broken = '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            // \u81ea\u5b9a\u4e49 JNA jar \u5185\u7f6e\u4e86 android-x86/x86_64/arm/arm64 \u5e73\u53f0\u7684 libjnidispatch.so\n            implementation(files("\''\\''/../../target/jna-5.13.0-android.jar"))\n                // \u7559\u4f5c\u5360\u4f4d,\u5b9e\u9645\u7531 repositories \u4e2d\u7684 flatDir \u66ff\u6362\n            }\n            implementation(libs.compose.material.icons.extended)'
replacement = '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            // \u81ea\u5b9a\u4e49 JNA jar \u5185\u7f6e\u4e86 android-x86/x86_64/arm/arm64 \u5e73\u53f0\u7684 libjnidispatch.so\n            implementation(files("\''\\''/../../target/jna-5.13.0-android.jar"))\n            implementation(libs.compose.material.icons.extended)'
print('broken block found:', broken in text)
text = text.replace(broken, replacement)
p.write_text(text, encoding='gbk')
# verify
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines()[20:50], start=20):
    print(f'{i+1}: {line!r}')

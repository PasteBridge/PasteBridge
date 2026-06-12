# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='utf-8')
old = '            // UniFFI \u5728 Android \u7aef\u4f7f\u7528 JNA \u4e0e Rust \u901a\u4fe1\n            implementation("net.java.dev.jna:jna:5.13.0")'
print('searching for:')
print(repr(old))
print('found:', old in text)
# Find the line with jna:5.13.0
for i, line in enumerate(text.splitlines()):
    if 'jna:5.13.0' in line:
        print(f'line {i}: {line!r}')
        # Check surrounding lines
        lines = text.splitlines()
        for j in range(max(0,i-3), min(len(lines), i+3)):
            print(f'  ctx {j}: {lines[j]!r}')

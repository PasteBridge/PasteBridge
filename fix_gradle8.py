# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
# Find and fix the line containing jna-5.13.0-android
lines = text.splitlines()
for i, line in enumerate(lines):
    if 'jna-5.13.0-android' in line:
        print(f'before: {i+1}: {line!r}')
        lines[i] = '            implementation(files("\/../../target/jna-5.13.0-android.jar"))'
        print(f'after:  {i+1}: {lines[i]!r}')
        break
p.write_text('\n'.join(lines) + '\n', encoding='gbk')

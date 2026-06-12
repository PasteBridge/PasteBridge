# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Write the literal with raw string
new_line = r'            implementation(files("/../../target/jna-5.13.0-android.jar"))'
for i, line in enumerate(lines):
    if 'jna-5.13.0-android' in line:
        lines[i] = new_line
        print(f'fixed: {i+1}: {lines[i]!r}')
        break
p.write_text('\n'.join(lines) + '\n', encoding='gbk')

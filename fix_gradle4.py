# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Insert 'androidMain.dependencies {' before the JNA comment block
# Currently lines[34] is '       }' (closing withHostTest)
# lines[35] is the JNA comment
# We need to insert '        androidMain.dependencies {' at index 35 (0-indexed)
new_lines = lines[:35] + ['        androidMain.dependencies {'] + lines[35:]
p.write_text('\n'.join(new_lines) + '\n', encoding='gbk')
# Verify
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines()[30:60], start=30):
    print(f'{i+1}: {line!r}')

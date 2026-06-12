# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Line 35 (0-indexed 34) is the orphan JNA comment - remove it
# We want line 35 to be '        androidMain.dependencies {'
del lines[34]
p.write_text('\n'.join(lines) + '\n', encoding='gbk')
# Verify
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines()[30:55], start=30):
    print(f'{i+1}: {line!r}')

# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Delete line 38 (placeholder comment) and line 39 (stray close brace)
# Use 0-indexed: 37, 38
del lines[38]
del lines[37]
p.write_text('\n'.join(lines) + '\n', encoding='gbk')
# verify
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines()[20:50], start=20):
    print(f'{i+1}: {line!r}')

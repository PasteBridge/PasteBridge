# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
for i, line in enumerate(lines):
    if 'jna-5.13.0' in line:
        print(f'{i+1}: {line!r}')
        print(f'  bytes: {line.encode("gbk")}')

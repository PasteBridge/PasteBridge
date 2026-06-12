# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
for i, line in enumerate(text.splitlines()):
    if 'jna' in line.lower():
        print(f'line {i}: {line!r}')

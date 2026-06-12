# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Find the line with 'androidRuntimeClasspath' to understand the structure
for i, line in enumerate(lines):
    if 'androidRuntimeClasspath' in line:
        print(f'androidRuntimeClasspath at line {i+1}: {line!r}')
        for j in range(max(0,i-5), min(len(lines), i+5)):
            print(f'  ctx {j+1}: {lines[j]!r}')

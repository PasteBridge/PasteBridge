# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Remove the old 'implementation("net.java.dev.jna:jna:5.13.0")' on line 41
# and its preceding comment on 40
for i, line in enumerate(lines):
    if 'net.java.dev.jna:jna:5.13.0' in line:
        # remove this line and the preceding comment
        del lines[i]
        del lines[i-1]
        break
p.write_text('\n'.join(lines) + '\n', encoding='gbk')
# verify
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines()):
    if 'jna' in line.lower() or 'UniFFI' in line:
        print(f'line {i}: {line!r}')

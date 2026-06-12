# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Fix line 42 (0-indexed 41):  -> 
# Fix line 64 (0-indexed 63): \\ ->  (remove the backslash)
# Fix line 65 (0-indexed 64): same
# Use byte-level operations via .encode
# Simpler: build the new line from chr()
D = chr(36)  # $
for i in [41, 63, 64]:
    if 'jna-5.13.0' in lines[i]:
        # Extract the path part
        old_line = lines[i]
        # Build new by replacing the bad path
        if 'rootDir' in old_line:
            lines[i] = old_line.replace('rootDir', 'rootDir')
        elif chr(92) + 'rootDir' in old_line.encode('gbk').decode('gbk'):
            # Has literal backslash before rootDir - remove
            lines[i] = old_line.replace(chr(92), '', 1)
        print(f'fixed: {i+1}: {lines[i]!r}')
p.write_text('\n'.join(lines) + '\n', encoding='gbk')
# verify
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    if 'jna-5.13.0' in line:
        print(f'{i}: {line!r}')

# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# Find and fix lines 64, 65 (the broken file paths)
for i, line in enumerate(lines):
    if 'jna-5.13.0-android.jar' in line and '/../../target' in line:
        # Need to add chr(36) = $ back
        # Replace '(files("' with '(files("' + chr(36) + 'rootDir'
        new = line.replace('files("', 'files("' + chr(36) + 'rootDir', 1)
        lines[i] = new
        print(f'fixed: {i+1}: {lines[i]!r}')
p.write_text('\n'.join(lines) + '\n', encoding='gbk')

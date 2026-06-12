# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
# Remove the literal backslash from the file paths
text = text.replace('"\\\\/../../target', '"' + chr(36) + 'rootDir/../../target')
# Wait this is wrong, let me use byte level
lines = text.splitlines()
for i, line in enumerate(lines):
    if 'jna-5.13.0' in line:
        if 'rootDir' in line:
            # Double rootDir
            lines[i] = line.replace('', chr(36) + 'rootDir')
            print(f'double fix {i+1}: {lines[i]!r}')
        # Look for literal backslash after 
        bslash_pos = line.find(chr(36) + 'rootDir' + chr(92))  # \
        if bslash_pos >= 0:
            new_line = line[:bslash_pos + 7] + line[bslash_pos + 8:]
            lines[i] = new_line
            print(f'bslash fix {i+1}: {lines[i]!r}')
p.write_text('\n'.join(lines) + '\n', encoding='gbk')
# verify
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    if 'jna-5.13.0' in line:
        print(f'{i}: {line!r}')

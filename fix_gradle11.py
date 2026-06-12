# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
for i, line in enumerate(lines):
    if 'jna-5.13.0' in line:
        new = '            implementation(files(' + chr(34) + chr(36) + 'rootDir/../../target/jna-5.13.0-android.jar' + chr(34) + '))'
        lines[i] = new
        print(f'fixed: {i+1}: {lines[i]!r}')
        break
p.write_text('\n'.join(lines) + '\n', encoding='gbk')

# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
# Fix the broken line
text = text.replace(
    'implementation(files("\'\''\\\\\'\'/../../target/jna-5.13.0-android.jar"))',
    'implementation(files("\/../../target/jna-5.13.0-android.jar"))'
)
p.write_text(text, encoding='gbk')
# verify line 42
text2 = p.read_text(encoding='gbk')
for i, line in enumerate(text2.splitlines(), start=1):
    if 'jna-5.13.0-android' in line:
        print(f'{i}: {line!r}')

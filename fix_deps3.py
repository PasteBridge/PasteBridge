# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
# Fix line 64, 65 with backslash issue
text = text.replace(
    '    implementation(files("\\/../../target/jna-5.13.0-android.jar"))',
    '    implementation(files("' + chr(36) + 'rootDir/../../target/jna-5.13.0-android.jar"))'
)
text = text.replace(
    '    runtimeOnly(files("\\/../../target/jna-5.13.0-android.jar"))',
    '    runtimeOnly(files("' + chr(36) + 'rootDir/../../target/jna-5.13.0-android.jar"))'
)
# Also fix line 42 (which has double )
text = text.replace(
    '            implementation(files("/../../target/jna-5.13.0-android.jar"))',
    '            implementation(files("' + chr(36) + 'rootDir/../../target/jna-5.13.0-android.jar"))'
)
p.write_text(text, encoding='gbk')
# verify
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    if 'jna-5.13.0' in line:
        print(f'{i}: {line!r}')

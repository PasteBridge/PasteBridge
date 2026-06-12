# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
# Add a top-level dependencies block with androidRuntimeClasspath-style dep
# Replace 'androidRuntimeClasspath(libs.compose.uiTooling)' with two deps
old = 'dependencies {\n    androidRuntimeClasspath(libs.compose.uiTooling)\n}'
new_deps = '''dependencies {
    androidRuntimeClasspath(libs.compose.uiTooling)
    // 自定义 JNA jar 内置了 android-x86/x86_64/arm/arm64 平台的 libjnidispatch.so
    implementation(files("\/../../target/jna-5.13.0-android.jar"))
    runtimeOnly(files("\/../../target/jna-5.13.0-android.jar"))
}'''
print('old found:', old in text)
text = text.replace(old, new_deps)
p.write_text(text, encoding='gbk')
# verify
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    if 'jna' in line.lower() or 'dependencies' in line.lower() or 'classpath' in line.lower():
        print(f'{i}: {line!r}')

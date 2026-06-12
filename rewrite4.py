# -*- coding: utf-8 -*-
import pathlib, zipfile, os
# Use the standard JNA jar from gradle cache
src_jar = pathlib.Path('C:/Users/Administrator/.gradle/caches/modules-2/files-2.1/net.java.dev.jna/jna/5.13.0/1200e7ebeedbe0d10062093f32925a912020e747/jna-5.13.0.jar')
out_jar = pathlib.Path('target/jna-5.13.0-android-only.jar')
mapping = [
    'com/sun/jna/android-x86/libjnidispatch.so',
    'com/sun/jna/android-x86-64/libjnidispatch.so',
    'com/sun/jna/android-aarch64/libjnidispatch.so',
    'com/sun/jna/android-arm/libjnidispatch.so',
]
with zipfile.ZipFile(src_jar) as src, zipfile.ZipFile(out_jar, 'w', zipfile.ZIP_DEFLATED) as out:
    for entry in mapping:
        out.writestr(entry, src.read(entry))
print('created', out_jar, out_jar.stat().st_size)

# Also rebuild the all-in-one jar (with everything + android dispatchers) for safety
out_jar_all = pathlib.Path('target/jna-5.13.0-android-all.jar')
with zipfile.ZipFile(src_jar) as src, zipfile.ZipFile(out_jar_all, 'w', zipfile.ZIP_DEFLATED) as out:
    for name in src.namelist():
        if any(name == m for m in mapping):
            continue
        out.writestr(name, src.read(name))
    for entry in mapping:
        out.writestr(entry, src.read(entry))
print('created', out_jar_all, out_jar_all.stat().st_size)

# Update build script
D = chr(36)
lines = [
    'import org.jetbrains.kotlin.gradle.dsl.JvmTarget',
    '',
    'plugins {',
    '    alias(libs.plugins.kotlinMultiplatform)',
    '    alias(libs.plugins.androidMultiplatformLibrary)',
    '    alias(libs.plugins.composeMultiplatform)',
    '    alias(libs.plugins.composeCompiler)',
    '}',
    '',
    'kotlin {',
    '    listOf(',
    '        iosArm64(),',
    '        iosSimulatorArm64()',
    '    ).forEach { iosTarget ->',
    '        iosTarget.binaries.framework {',
    '            baseName = "Shared"',
    '            isStatic = true',
    '        }',
    '    }',
    '',
    '    androidLibrary {',
    '       namespace = "org.pastebridge.app.shared"',
    '       compileSdk = libs.versions.android.compileSdk.get().toInt()',
    '       minSdk = libs.versions.android.minSdk.get().toInt()',
    '',
    '       compilerOptions {',
    '           jvmTarget = JvmTarget.JVM_11',
    '       }',
    '       androidResources {',
    '           enable = true',
    '       }',
    '       withHostTest {',
    '           isIncludeAndroidResources = true',
    '       }',
    '    }',
    '',
    '    sourceSets {',
    '        androidMain.dependencies {',
    '            implementation(libs.compose.uiToolingPreview)',
    '            // UniFFI 在 Android 端使用 JNA 与 Rust 通信',
    '            implementation("net.java.dev.jna:jna:5.13.0")',
    '            // 额外注入 android 平台的 libjnidispatch.so,JNA 会从 classpath 加载',
    '            implementation(files("' + D + 'rootDir/../../target/jna-5.13.0-android-only.jar"))',
    '            implementation(libs.compose.material.icons.extended)',
    '        }',
    '        commonMain.dependencies {',
    '            implementation(libs.compose.runtime)',
    '            implementation(libs.compose.foundation)',
    '            implementation(libs.compose.material3)',
    '            implementation(libs.compose.ui)',
    '            implementation(libs.compose.components.resources)',
    '            implementation(libs.compose.uiToolingPreview)',
    '            implementation(libs.androidx.lifecycle.viewmodelCompose)',
    '            implementation(libs.androidx.lifecycle.runtimeCompose)',
    '        }',
    '        commonTest.dependencies {',
    '            implementation(libs.kotlin.test)',
    '        }',
    '    }',
    '}',
    '',
    'dependencies {',
    '    androidRuntimeClasspath(libs.compose.uiTooling)',
    '}',
    '',
]
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
p.write_text('\n'.join(lines), encoding='gbk')
print('rewritten')

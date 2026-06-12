# -*- coding: utf-8 -*-
import pathlib
# Write a properly-formatted file from scratch using a different approach
# Build the content piece by piece, with  handled carefully

# Use a string for the dollar sign so Python doesn't interpret it
ROOT = chr(36) + 'rootDir'  # ''
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
    '            // 自定义 JNA jar 内置了 android-x86/x86_64/arm/arm64 平台的 libjnidispatch.so',
    '            implementation("net.java.dev.jna:jna:5.13.0")',
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
    '    // 用本地 jar 替换标准 jna:5.13.0 来注入 android 平台的 libjnidispatch.so',
    '    implementation(files("' + D + 'rootDir/../../target/jna-5.13.0-android.jar"))',
    '    runtimeOnly(files("' + D + 'rootDir/../../target/jna-5.13.0-android.jar"))',
    '}',
    '',
]

p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
p.write_text('\n'.join(lines), encoding='gbk')
print('rewritten, total lines:', len(lines))
# verify
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    if 'jna' in line.lower() or 'dependencies' in line.lower():
        print(f'{i}: {line!r}')

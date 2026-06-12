# -*- coding: utf-8 -*-
import pathlib
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
    '            // 在依赖顶层用本地 jar 替换标准 jna:5.13.0 以注入 android 平台的 libjnidispatch.so',
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
    '    "implementation"(files("' + D + 'rootDir/../../target/jna-5.13.0-android.jar"))',
    '    "implementation"(files("' + D + 'rootDir/../../target/jna-5.13.0-android.jar"))',
    '}',
    '',
]
# Actually implementation isn't allowed in top-level. Use configurations:
# 'androidMainImplementation' or directly target the configuration name.
# In KMP androidLibrary, the correct way is to add a configuration. Let me use:
# add('androidMainImplementation', files(...))
# Or simply put it back in androidMain.dependencies block
# But the issue is files() in androidMain.dependencies wasn't taking effect.

# Better approach: write to a separate jniLibsPrep task that copies the dispatcher.
# Actually the simplest: extract libjnidispatch.so from our custom jar and put in
# the jniLibs/x86 and jniLibs/x86_64 directories of the source set.

# Let me update the script: extract the .so from the custom jar and put in jniLibs
import zipfile, os, shutil
src_jar = pathlib.Path('target/jna-5.13.0-android.jar')
out_dirs = {
    'com/sun/jna/android-x86/libjnidispatch.so': 'crates/mobile/shared/src/androidMain/jniLibs/x86/libjnidispatch.so',
    'com/sun/jna/android-x86-64/libjnidispatch.so': 'crates/mobile/shared/src/androidMain/jniLibs/x86_64/libjnidispatch.so',
    'com/sun/jna/android-aarch64/libjnidispatch.so': 'crates/mobile/shared/src/androidMain/jniLibs/arm64-v8a/libjnidispatch.so',
    'com/sun/jna/android-arm/libjnidispatch.so': 'crates/mobile/shared/src/androidMain/jniLibs/armeabi-v7a/libjnidispatch.so',
}
z = zipfile.ZipFile(src_jar)
for src_name, dst in out_dirs.items():
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    with z.open(src_name) as f, open(dst, 'wb') as g:
        g.write(f.read())
    print('wrote', dst, os.path.getsize(dst), 'bytes')
z.close()

# Now write the build script with the standard JNA dep + extra jniLibs
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

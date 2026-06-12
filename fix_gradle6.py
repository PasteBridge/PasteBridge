# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
text = p.read_text(encoding='gbk')
lines = text.splitlines()
# We need to:
# 1. Remove lines 35-52 (the misplaced sourceSets content inside androidLibrary)
# 2. Close androidLibrary properly at line 35 (just '}' after withHostTest)
# 3. Add a new sourceSets block as sibling to androidLibrary inside kotlin { }
# 4. Close kotlin { } properly

# Actually the cleanest fix: rewrite the whole file properly
new_text = '''import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidMultiplatformLibrary)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
}

kotlin {
    listOf(
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "Shared"
            isStatic = true
        }
    }

    androidLibrary {
       namespace = "org.pastebridge.app.shared"
       compileSdk = libs.versions.android.compileSdk.get().toInt()
       minSdk = libs.versions.android.minSdk.get().toInt()

       compilerOptions {
           jvmTarget = JvmTarget.JVM_11
       }
       androidResources {
           enable = true
       }
       withHostTest {
           isIncludeAndroidResources = true
       }
    }

    sourceSets {
        androidMain.dependencies {
            implementation(libs.compose.uiToolingPreview)
            // UniFFI 在 Android 端使用 JNA 与 Rust 通信
            // 自定义 JNA jar 内置了 android-x86/x86_64/arm/arm64 平台的 libjnidispatch.so
            implementation(files("\''\\''/../../target/jna-5.13.0-android.jar"))
            implementation(libs.compose.material.icons.extended)
        }
        commonMain.dependencies {
            implementation(libs.compose.runtime)
            implementation(libs.compose.foundation)
            implementation(libs.compose.material3)
            implementation(libs.compose.ui)
            implementation(libs.compose.components.resources)
            implementation(libs.compose.uiToolingPreview)
            implementation(libs.androidx.lifecycle.viewmodelCompose)
            implementation(libs.androidx.lifecycle.runtimeCompose)
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
        }
    }
}

dependencies {
    androidRuntimeClasspath(libs.compose.uiTooling)
}
'''
p.write_text(new_text, encoding='gbk')
print('rewritten')
for i, line in enumerate(p.read_text(encoding='gbk').splitlines(), start=1):
    print(f'{i:3}: {line!r}')

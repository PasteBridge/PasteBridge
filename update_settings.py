# -*- coding: utf-8 -*-
import pathlib
D = chr(36)
settings = pathlib.Path('crates/mobile/settings.gradle.kts')
text = settings.read_text(encoding='gbk')
# Add local maven repo before mavenCentral() in dependencyResolutionManagement
old = '    mavenCentral()\n        maven { url = uri("https://maven.pkg.jetbrains.space/public/p/compose/dev") }'
new = '    maven { url = uri("' + D + 'rootDir/../../target/maven") }\n        mavenCentral()\n        maven { url = uri("https://maven.pkg.jetbrains.space/public/p/compose/dev") }'
print('old found:', old in text)
text = text.replace(old, new)
settings.write_text(text, encoding='gbk')
print('settings updated')
print(settings.read_text(encoding='gbk'))

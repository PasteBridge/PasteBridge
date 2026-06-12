# -*- coding: utf-8 -*-
import pathlib
# Add a local maven repo to settings.gradle.kts with the patched JNA
settings = pathlib.Path('crates/mobile/settings.gradle.kts')
text = settings.read_text(encoding='gbk')
print('current settings:')
print(text)
# Add a local repo to pluginManagement and dependencyResolutionManagement
# Place the jar in a maven-style structure: target/maven/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar
import shutil
maven_dir = pathlib.Path('target/maven/net/java/dev/jna/jna/5.13.0')
maven_dir.mkdir(parents=True, exist_ok=True)
shutil.copy('target/jna-5.13.0.jar', maven_dir / 'jna-5.13.0.jar')
# Also create a pom
pom = maven_dir / 'jna-5.13.0.pom'
pom.write_text('''<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>net.java.dev.jna</groupId>
    <artifactId>jna</artifactId>
    <version>5.13.0</version>
    <packaging>jar</packaging>
    <name>Java Native Access (patched with android dispatchers)</name>
</project>
''', encoding='utf-8')
print('maven repo at', maven_dir.parent.parent.parent.parent.parent)

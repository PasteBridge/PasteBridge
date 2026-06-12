# -*- coding: utf-8 -*-
import pathlib
# Use the original full POM from the Maven Central download
src = pathlib.Path('C:/Users/Administrator/.gradle/caches/modules-2/files-2.1/net.java.dev.jna/jna/5.13.0/b7cc05a5394544befc936c39080a93cc8c1e082e/jna-5.13.0.pom')
dst = pathlib.Path('target/maven/net/java/dev/jna/jna/5.13.0/jna-5.13.0.pom')
import shutil
shutil.copy(src, dst)
print('Copied full POM:')
print(dst.read_text(encoding='utf-8'))

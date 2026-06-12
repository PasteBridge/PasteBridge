# -*- coding: utf-8 -*-
import pathlib, zipfile, os, shutil
# Make a fully patched JNA jar that has everything + android dispatchers
src_jar = pathlib.Path('C:/Users/Administrator/.gradle/caches/modules-2/files-2.1/net.java.dev.jna/jna/5.13.0/1200e7ebeedbe0d10062093f32925a912020e747/jna-5.13.0.jar')
out_jar = pathlib.Path('target/jna-5.13.0.jar')  # Same name as standard
mapping = {
    'com/sun/jna/linux-x86/libjnidispatch.so': 'com/sun/jna/android-x86/libjnidispatch.so',
    'com/sun/jna/linux-x86-64/libjnidispatch.so': 'com/sun/jna/android-x86-64/libjnidispatch.so',
    'com/sun/jna/linux-aarch64/libjnidispatch.so': 'com/sun/jna/android-aarch64/libjnidispatch.so',
    'com/sun/jna/linux-arm/libjnidispatch.so': 'com/sun/jna/android-arm/libjnidispatch.so',
}
with zipfile.ZipFile(src_jar) as src, zipfile.ZipFile(out_jar, 'w', zipfile.ZIP_DEFLATED) as out:
    for name in src.namelist():
        out.writestr(name, src.read(name))
    for src_name, dst_name in mapping.items():
        out.writestr(dst_name, src.read(src_name))
        print(f'  added {dst_name}')
print('created', out_jar, out_jar.stat().st_size, 'bytes')

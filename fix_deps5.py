# -*- coding: utf-8 -*-
import pathlib
p = pathlib.Path('crates/mobile/shared/build.gradle.kts')
data = p.read_bytes()
# GBK encoded, decode then re-encode
text = data.decode('gbk')
lines = text.splitlines()
# Show the raw bytes for line 64
for i, line in enumerate(lines):
    if 'jna-5.13.0' in line and '/../../target' in line and '\\' in line:
        print(f'line {i+1}:')
        for ch in line:
            if ord(ch) > 127 or ch in '$\\/':
                print(f'  char: {ch!r} (0x{ord(ch):04x})')

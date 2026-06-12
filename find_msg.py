# -*- coding: utf-8 -*-
import pathlib, re
p = pathlib.Path('crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/paste_bridge_core.kt')
text = p.read_text(encoding='utf-8')
# search for the actual hex of the backticks around message
for i, ch in enumerate(text):
    if ch == chr(0x60):
        ctx = text[max(0,i-5):i+10]
        if 'message' in ctx:
            print(i, repr(ctx))

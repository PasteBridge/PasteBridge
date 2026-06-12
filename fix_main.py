import pathlib
p = pathlib.Path("crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt")
t = p.read_text("utf-8")
t = t.replace('System.setProperty("jna.nolibrary", "true")', 'System.loadLibrary("jnidispatch")')
p.write_text(t, "utf-8")
print("ok")

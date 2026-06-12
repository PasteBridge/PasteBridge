import pathlib
p = pathlib.Path("crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt")
t = p.read_text("utf-8")
new_init = '''    companion object {
        init {
            // Preload JNA native dispatcher from APK native libs (lib/<abi>/libjnidispatch.so)
            System.loadLibrary("jnidispatch")
        }
    }'''
# Find the companion object and replace its contents
old_start = t.find("companion object")
old_end = t.find("}", old_start) + 1
old_block = t[old_start:old_end]
t = t.replace(old_block, new_init)
p.write_text(t, "utf-8")
print("ok")

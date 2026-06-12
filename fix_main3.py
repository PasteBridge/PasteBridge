import pathlib
p = pathlib.Path("crates/mobile/androidApp/src/main/kotlin/org/pastebridge/app/MainActivity.kt")
t = p.read_text("utf-8")

# The file has:
# class MainActivity : ComponentActivity() {
#         companion object {       <-- 8 spaces
#         init {
#             System.loadLibrary("jnidispatch")
#         }
#     }
#     }                             <-- extra closing brace
#     override fun onCreate...
#
# Need to: fix indentation of companion object, remove trailing brace

# Fix: remove the double close brace before onCreate
t = t.replace("    }\n    }\n    override fun onCreate", "    }\n    override fun onCreate")

# Fix indentation of companion object
t = t.replace("        companion object {", "    companion object {")
t = t.replace("        init {", "        init {")

p.write_text(t, "utf-8")
print("fixed")
print(p.read_text("utf-8"))

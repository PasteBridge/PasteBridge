import pathlib
p = pathlib.Path("crates/core/Cargo.toml")
c = p.read_text("utf-8")
# Add jni deps
c = c.replace('uniffi = "0.31"', 'uniffi = "0.31"')
# Instead, add after the ureq line
jni_deps = '\njni = { version = "0.21", optional = true }\njni-sys = { version = "0.3", optional = true }'
c = c.replace('ureq = { version = "2.10", default-features = false, features = ["json"] }', 'ureq = { version = "2.10", default-features = false, features = ["json"] }' + jni_deps)
# Add jni-bridge feature
c = c.replace('desktop-clipboard = ["dep:arboard"]', 'desktop-clipboard = ["dep:arboard"]\njni-bridge = ["dep:jni", "dep:jni-sys"]')
p.write_text(c, "utf-8")
print("updated Cargo.toml")

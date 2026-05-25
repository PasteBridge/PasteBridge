fn main() {
    // Let the MSVC environment (vcvars64) provide correct LIB paths and
    // avoid hardcoding specific MSVC toolset versions here.
    slint_build::compile("ui/app.slint").unwrap();
}

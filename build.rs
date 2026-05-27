fn main() {
    slint_build::compile("crates/desktop/ui/app.slint").unwrap();

    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    {
        if let Some(msvc_lib_dir) = find_highest_msvc_lib_dir() {
            println!("cargo:rustc-link-search={}", msvc_lib_dir);
        }
        if let Some(ucrt_lib_dir) = find_ucrt_lib_dir() {
            println!("cargo:rustc-link-search={}", ucrt_lib_dir);
        }

        let out_dir = std::path::PathBuf::from(
            std::env::var("OUT_DIR").expect("OUT_DIR not set"),
        );

        cc::Build::new()
            .cpp(true)
            .file("msvc_stubs.cpp")
            .compile("msvc_stubs");

        println!(
            "cargo:rustc-link-arg-bin=paste_bridge=/WHOLEARCHIVE:{}",
            out_dir.join("msvc_stubs.lib").display()
        );
    }
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn find_highest_msvc_lib_dir() -> Option<String> {
    let roots = [
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
        r"C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        r"C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC",
        r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Professional\VC\Tools\MSVC",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Enterprise\VC\Tools\MSVC",
    ];

    let mut best: Option<(u32, u32, u32, String)> = None;

    for root in &roots {
        let dir = std::path::Path::new(root);
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = entry.file_name();
                let name = match name.to_str() {
                    Some(n) => n,
                    None => continue,
                };
                let parts: Vec<&str> = name.split('.').collect();
                if parts.len() < 3 {
                    continue;
                }
                let a = parts[0].parse::<u32>().ok()?;
                let b = parts[1].parse::<u32>().ok()?;
                let c = parts[2].parse::<u32>().ok()?;

                let lib_dir = path.join("lib").join("x64");
                if lib_dir.exists() {
                    let should_replace = match &best {
                        Some((ba, bb, bc, _)) => {
                            a > *ba || (a == *ba && b > *bb) || (a == *ba && b == *bb && c > *bc)
                        }
                        None => true,
                    };
                    if should_replace {
                        best = Some((a, b, c, lib_dir.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    best.map(|(_, _, _, path)| path)
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn find_ucrt_lib_dir() -> Option<String> {
    let kits_root = r"C:\Program Files (x86)\Windows Kits\10\Lib";
    let kits = std::path::Path::new(kits_root);
    if !kits.exists() {
        return None;
    }

    let mut best_version: Option<(u32, u32, u32, String)> = None;

    if let Ok(entries) = std::fs::read_dir(kits) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = match name.to_str() {
                Some(n) => n,
                None => continue,
            };
            let parts: Vec<&str> = name.split('.').collect();
            if parts.len() < 3 {
                continue;
            }
            let a = parts[0].parse::<u32>().ok()?;
            let b = parts[1].parse::<u32>().ok()?;
            let c = parts[2].parse::<u32>().ok()?;

            let ucrt_dir = path.join("ucrt").join("x64");
            if ucrt_dir.exists() {
                let should_replace = match &best_version {
                    Some((ba, bb, bc, _)) => {
                        a > *ba || (a == *ba && b > *bb) || (a == *ba && b == *bb && c > *bc)
                    }
                    None => true,
                };
                if should_replace {
                    best_version = Some((a, b, c, ucrt_dir.to_string_lossy().to_string()));
                }
            }
        }
    }

    best_version.map(|(_, _, _, path)| path)
}
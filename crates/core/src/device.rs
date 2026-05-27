use std::fs;
use std::path::PathBuf;

pub fn get_device_id(app_data_dir: &PathBuf) -> String {
    let device_file = app_data_dir.join("device.id");

    if let Ok(uuid) = fs::read_to_string(&device_file) {
        let uuid = uuid.trim().to_string();
        if !uuid.is_empty() {
            return uuid;
        }
    }

    if let Some(uuid) = get_machine_guid() {
        fs::create_dir_all(app_data_dir).ok();
        let _ = fs::write(&device_file, &uuid);
        return uuid;
    }

    let uuid = generate_fallback_uuid();
    fs::create_dir_all(app_data_dir).ok();
    let _ = fs::write(&device_file, &uuid);
    uuid
}

fn get_machine_guid() -> Option<String> {
    use std::process::Command;

    let output = Command::new("reg")
        .args([
            "query",
            r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("MachineGuid") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                return Some(parts[parts.len() - 1].to_string());
            }
        }
    }
    None
}

fn generate_fallback_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let random: u32 = rand_simple();

    format!("{:x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (now & 0xFFFFFFFF) as u32,
        ((now >> 32) & 0xFFFF) as u16,
        (((now >> 48) & 0xFFFF) as u16) | 0x4000,
        ((random & 0x3FFF) | 0x8000),
        ((random as u64) << 48) | ((now % 0xFFFFFFFFFFFF) as u64)
    )
}

fn rand_simple() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let mut x = (nanos as u32).wrapping_mul(747796405);
    x ^= x >> 17;
    x ^= x << 5;
    x ^= x >> 13;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_generation() {
        let temp_dir = std::env::temp_dir().join("pastebridge_test");
        let id1 = get_device_id(&temp_dir);
        let id2 = get_device_id(&temp_dir);

        assert_eq!(id1, id2);

        let _ = fs::remove_file(temp_dir.join("device.id"));
    }
}
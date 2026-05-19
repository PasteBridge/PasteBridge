use std::fs;
use std::path::PathBuf;

/// 获取设备唯一 ID
/// 优先使用本地存储的 UUID，否则读取 MachineGuid，最终生成新 UUID
pub fn get_device_id(app_data_dir: &PathBuf) -> String {
    let device_file = app_data_dir.join("device.id");

    // 1. 尝试读取已保存的设备 ID
    if let Ok(uuid) = fs::read_to_string(&device_file) {
        let uuid = uuid.trim().to_string();
        if !uuid.is_empty() {
            return uuid;
        }
    }

    // 2. 回退: 读取 Windows MachineGuid
    if let Some(uuid) = get_machine_guid() {
        // 保存作为备份
        fs::create_dir_all(app_data_dir).ok();
        let _ = fs::write(&device_file, &uuid);
        return uuid;
    }

    // 3. 生成新 UUID (需要 uuid crate，这里先用时间戳+随机数)
    let uuid = generate_fallback_uuid();
    fs::create_dir_all(app_data_dir).ok();
    let _ = fs::write(&device_file, &uuid);
    uuid
}

/// 从注册表读取 MachineGuid
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

/// 生成备用 UUID（基于时间戳 + 随机数）
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
        (((now >> 48) & 0xFFFF) as u16) | 0x4000, // version 4
        ((random & 0x3FFF) | 0x8000), // random
        ((random as u64) << 48) | ((now % 0xFFFFFFFFFFFF) as u64)
    )
}

/// 简单随机数生成（不依赖外部库）
fn rand_simple() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    // xorshift32
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
        
        assert_eq!(id1, id2); // 应该相同（已保存）
        
        // 清理
        let _ = fs::remove_file(temp_dir.join("device.id"));
    }
}

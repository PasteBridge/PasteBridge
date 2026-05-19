//! Hotkey management


/// Hotkey identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyId(pub u32);

/// Register a global hotkey
/// Returns HotkeyId on success
pub fn register_hotkey(
    _key: &str,
    _modifiers: &[&str],
) -> Result<HotkeyId, String> {
    // Placeholder - actual implementation in platform-specific code
    Err("Not implemented".to_string())
}

/// Unregister a hotkey
pub fn unregister_hotkey(_id: HotkeyId) -> Result<(), String> {
    // Placeholder
    Ok(())
}

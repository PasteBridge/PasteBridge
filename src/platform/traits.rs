//! Platform traits - define interfaces for cross-platform support


/// Window handle type
pub type RawHandle = isize;

/// Trait for platform-specific window operations
pub trait PlatformWindow {
    /// Show the window at calculated position
    fn show(&self) -> Result<(), String>;
    
    /// Hide the window off-screen
    fn hide(&self) -> Result<(), String>;
    
    /// Fade in the window
    fn fade_in(&self, _hwnd: RawHandle) -> Result<(), String> {
        Ok(())
    }
    
    /// Fade out the window
    fn fade_out(&self, _hwnd: RawHandle) -> Result<(), String> {
        Ok(())
    }
    
    /// Set window position
    fn set_position(&self, x: i32, y: i32) -> Result<(), String>;
    
    /// Get window handle
    fn get_hwnd(&self) -> RawHandle;
}

/// Trait for platform-specific clipboard operations
pub trait PlatformClipboard {
    /// Get current clipboard text
    fn get_text(&self) -> Option<String>;
    
    /// Set clipboard text
    fn set_text(&self, text: &str) -> Result<(), String>;
}

/// Trait for platform-specific hotkey operations
pub trait PlatformHotkeyTrait {
    /// Register a hotkey
    fn register(&mut self, key: &str, modifiers: &[&str]) -> Result<u32, String>;
    
    /// Unregister a hotkey
    fn unregister(&mut self, id: u32) -> Result<(), String>;
}

/// Trait for platform-specific tray operations
pub trait PlatformTrayTrait {
    /// Setup tray icon
    fn setup(&self) -> Result<TrayHandles, String>;
    
    /// Set tray tooltip
    fn set_tooltip(&self, tooltip: &str) -> Result<(), String>;
}

/// Tray handles
pub struct TrayHandles {
    pub show_id: String,
    pub quit_id: String,
}

/// Tray icon handle
pub struct TrayIconHandle {
    _priv: (),
}

pub type RawHandle = isize;

pub trait PlatformWindow {
    fn show(&self) -> Result<(), String>;

    fn hide(&self) -> Result<(), String>;

    fn fade_in(&self, _hwnd: RawHandle) -> Result<(), String> {
        Ok(())
    }

    fn fade_out(&self, _hwnd: RawHandle) -> Result<(), String> {
        Ok(())
    }

    fn set_position(&self, x: i32, y: i32) -> Result<(), String>;

    fn get_hwnd(&self) -> RawHandle;
}

pub trait PlatformClipboard {
    fn get_text(&self) -> Option<String>;

    fn set_text(&self, text: &str) -> Result<(), String>;
}

pub trait PlatformHotkeyTrait {
    fn register(&mut self, key: &str, modifiers: &[&str]) -> Result<u32, String>;

    fn unregister(&mut self, id: u32) -> Result<(), String>;
}

pub trait PlatformTrayTrait {
    fn setup(&self) -> Result<TrayHandles, String>;

    fn set_tooltip(&self, tooltip: &str) -> Result<(), String>;
}

pub struct TrayHandles {
    pub show_id: String,
    pub quit_id: String,
}

pub struct TrayIconHandle {
    _priv: (),
}
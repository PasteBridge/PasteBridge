use std::cell::RefCell;
use arboard::Clipboard;
use crate::platform::traits::PlatformClipboard;

pub struct WindowsClipboard {
    clipboard: RefCell<Option<Clipboard>>,
}

impl WindowsClipboard {
    pub fn new() -> Result<Self, String> {
        let clipboard = Clipboard::new()
            .map_err(|e| format!("Failed to create clipboard: {}", e))?;
        Ok(Self {
            clipboard: RefCell::new(Some(clipboard)),
        })
    }
}

impl PlatformClipboard for WindowsClipboard {
    fn get_text(&self) -> Option<String> {
        self.clipboard.borrow_mut().as_mut().and_then(|c| c.get_text().ok())
    }

    fn set_text(&self, text: &str) -> Result<(), String> {
        self.clipboard.borrow_mut().as_mut()
            .ok_or_else(|| "Clipboard not initialized".to_string())?
            .set_text(text)
            .map_err(|e| e.to_string())
    }
}
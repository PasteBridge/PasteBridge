//! Application state shared across modules

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use crate::core::database::{Database, ClipboardItem};

/// Shared application state
pub struct AppState {
    /// Database for persistent storage
    pub db: Mutex<Database>,
    /// Maximum history size
    pub max_history_size: usize,
    /// Window visibility state
    pub is_window_visible: Mutex<bool>,
    /// Device ID
    pub device_id: String,
}

impl AppState {
    /// Create new AppState with database
    pub fn new(app_data_dir: &PathBuf, max_history_size: usize) -> Arc<Self> {
        let db_path = app_data_dir.join("clipboard.db");
        let images_dir = app_data_dir.join("images");
        
        let db = Database::new(&db_path, &images_dir)
            .expect("Failed to open database");
        
        let device_id = crate::core::device::get_device_id(app_data_dir);
        
        Arc::new(Self {
            db: Mutex::new(db),
            max_history_size,
            is_window_visible: Mutex::new(false),
            device_id,
        })
    }

    /// Add text to clipboard history (stores in database)
    pub fn push_clipboard(&self, text: String) -> Option<i64> {
        let db = self.db.lock().unwrap();
        db.insert_text(&text).ok()
    }

    /// Add image to clipboard history
    pub fn push_image(&self, data: &[u8], mime_type: &str, width: i32, height: i32) -> Option<(i64, String)> {
        let db = self.db.lock().unwrap();
        db.insert_image(data, mime_type, width, height).ok()
    }

    /// Get clipboard history from database
    pub fn get_history(&self) -> Vec<ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.get_history(self.max_history_size).unwrap_or_default()
    }

    /// Get single item by ID
    pub fn get_item(&self, id: i64) -> Option<ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.get_item(id).ok().flatten()
    }

    /// Delete clipboard item
    pub fn delete_item(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.delete(id).is_ok()
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.toggle_favorite(id).unwrap_or(false)
    }

    /// Clear non-favorite items
    pub fn clear_history(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.clear_non_favorites().unwrap_or(0)
    }

    /// Get total item count
    pub fn count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.count().unwrap_or(0)
    }

    /// Set window visibility
    pub fn set_window_visible(&self, visible: bool) {
        *self.is_window_visible.lock().unwrap() = visible;
    }

    /// Check window visibility
    pub fn is_window_visible(&self) -> bool {
        *self.is_window_visible.lock().unwrap()
    }

    /// Get device ID
    pub fn get_device_id(&self) -> &str {
        &self.device_id
    }
}

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use crate::database::Database;
use crate::models::{ClipboardItem, FavoriteFolder};
use crate::device;

pub struct AppState {
    pub db: Mutex<Database>,
    pub max_history_size: usize,
    pub is_window_visible: Mutex<bool>,
    pub device_id: String,
}

impl AppState {
    pub fn new(app_data_dir: &PathBuf, max_history_size: usize) -> Arc<Self> {
        let db_path = app_data_dir.join("clipboard.db");
        let images_dir = app_data_dir.join("images");

        let db = Database::new(&db_path, &images_dir)
            .expect("Failed to open database");

        let device_id = device::get_device_id(app_data_dir);

        Arc::new(Self {
            db: Mutex::new(db),
            max_history_size,
            is_window_visible: Mutex::new(false),
            device_id,
        })
    }

    pub fn push_clipboard(&self, text: String) -> Option<i64> {
        let db = self.db.lock().unwrap();
        db.insert_text(&text).ok()
    }

    pub fn push_image(&self, data: &[u8], mime_type: &str, width: i32, height: i32) -> Option<(i64, String)> {
        let db = self.db.lock().unwrap();
        db.insert_image(data, mime_type, width, height).ok()
    }

    pub fn get_history(&self, ascending: bool) -> Vec<ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.get_history(self.max_history_size, ascending).unwrap_or_default()
    }

    pub fn search_history(&self, query: &str, ascending: bool) -> Vec<ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.search_history(query, self.max_history_size, ascending).unwrap_or_default()
    }

    pub fn get_item(&self, id: i64) -> Option<ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.get_item(id).ok().flatten()
    }

    pub fn delete_item(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.delete(id).is_ok()
    }

    pub fn toggle_favorite(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.toggle_favorite(id).unwrap_or(false)
    }

    pub fn clear_history(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.clear_non_favorites().unwrap_or(0)
    }

    pub fn count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.count().unwrap_or(0)
    }

    pub fn add_mock_data(&self, count: usize) -> usize {
        let db = self.db.lock().unwrap();
        db.insert_mock_data(count).unwrap_or(0)
    }

    pub fn get_config(&self, key: &str) -> Option<String> {
        let db = self.db.lock().unwrap();
        db.get_config(key).ok().flatten()
    }

    pub fn set_config(&self, key: &str, value: &str) -> bool {
        let db = self.db.lock().unwrap();
        db.set_config(key, value).is_ok()
    }

    pub fn set_window_visible(&self, visible: bool) {
        *self.is_window_visible.lock().unwrap() = visible;
    }

    pub fn is_window_visible(&self) -> bool {
        *self.is_window_visible.lock().unwrap()
    }

    pub fn get_device_id(&self) -> &str {
        &self.device_id
    }

    // ========== Favorite Folders ==========

    pub fn get_favorite_folders(&self) -> Vec<FavoriteFolder> {
        let db = self.db.lock().unwrap();
        db.get_favorite_folders().unwrap_or_default()
    }

    pub fn insert_favorite_folder(&self, name: &str) -> Option<i64> {
        let db = self.db.lock().unwrap();
        db.insert_favorite_folder(name).ok()
    }

    pub fn rename_favorite_folder(&self, id: i64, name: &str) -> bool {
        let db = self.db.lock().unwrap();
        db.rename_favorite_folder(id, name).is_ok()
    }

    pub fn delete_favorite_folder(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.delete_favorite_folder(id).is_ok()
    }

    pub fn reorder_favorite_folders(&self, old_index: usize, new_index: usize) -> bool {
        let db = self.db.lock().unwrap();
        db.reorder_favorite_folders(old_index, new_index).is_ok()
    }

    pub fn add_item_to_favorite_folder(&self, folder_index: usize, item_id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.add_item_to_favorite_folder(folder_index, item_id).is_ok()
    }

    pub fn get_favorite_folder_item_count(&self, folder_id: i64) -> usize {
        let db = self.db.lock().unwrap();
        db.get_favorite_folder_item_count(folder_id).unwrap_or(0)
    }

    pub fn get_favorite_folder_items(&self, folder_index: usize) -> Vec<crate::models::ClipboardItem> {
        let db = self.db.lock().unwrap();
        db.get_favorite_folder_items(folder_index).unwrap_or_default()
    }

    pub fn init_default_favorite_folders(&self) {
        let db = self.db.lock().unwrap();
        if let Err(e) = db.init_default_favorite_folders() {
            eprintln!("[state] Failed to init default favorite folders: {}", e);
        }
    }

    /// 加载所有收藏夹及其真实条目数,供启动时一次性同步到 UI 使用。
    pub fn get_favorite_folders_with_count(&self) -> Vec<(FavoriteFolder, usize)> {
        let db = self.db.lock().unwrap();
        let folders = db.get_favorite_folders().unwrap_or_default();
        folders
            .into_iter()
            .map(|f| {
                let count = db.get_favorite_folder_item_count(f.id).unwrap_or(0);
                (f, count)
            })
            .collect()
    }

    pub fn add_item(&self, text: &str) -> i64 {
        let db = self.db.lock().unwrap();
        db.insert_text(text).unwrap_or(-1)
    }
}
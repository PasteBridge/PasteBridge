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

        let db = Database::new(&db_path)
            .expect("Failed to open database");

        // 旧 schema → 新 schema 迁移(幂等,首次启动将旧文件图片读进 BLOB)
        if let Err(e) = db.migrate_files_to_blob(app_data_dir) {
            eprintln!("[state] migration failed: {}; continuing with current schema", e);
        }

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

    /// 存储图片 (原图 + 缩略图) 到数据库 BLOB
    pub fn push_image(
        &self,
        data: &[u8],
        thumbnail: &[u8],
        mime_type: &str,
        width: i32,
        height: i32,
    ) -> Option<i64> {
        let db = self.db.lock().unwrap();
        db.insert_image(data, thumbnail, mime_type, width, height).ok()
    }

    /// 按 id 读取图片 BLOB 数据 (bytes, mime_type)
    pub fn get_image_data(&self, id: i64) -> Option<(Vec<u8>, String)> {
        let db = self.db.lock().unwrap();
        db.get_image_data(id).ok().flatten()
    }

    /// 按 id 读取缩略图 BLOB
    pub fn get_thumbnail_data(&self, id: i64) -> Option<Vec<u8>> {
        let db = self.db.lock().unwrap();
        db.get_thumbnail_data(id).ok().flatten()
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

    pub fn update_item_created_at(&self, id: i64) -> bool {
        let db = self.db.lock().unwrap();
        db.update_item_created_at(id).is_ok()
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

    /// 从 Ditto 剪贴板管理器导入文本条目。
    /// 返回 (导入成功数, 总条目数, 错误信息)。
    pub fn import_from_ditto(&self) -> (usize, usize, String) {
        // Ditto 数据库可能在 %LOCALAPPDATA% 或 %APPDATA% 下
        let ditto_db_path = {
            let candidates = [
                (
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Local")),
                    "Ditto",
                ),
                (
                    std::env::var("APPDATA")
                        .unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Roaming")),
                    "Ditto",
                ),
            ];

            let mut found = None;
            for (base, sub) in &candidates {
                let mut path = std::path::PathBuf::from(base);
                path.push(sub);
                path.push("Ditto.db");
                if path.exists() {
                    found = Some(path);
                    break;
                }
            }
            found
        };

        let ditto_db_path = match ditto_db_path {
            Some(p) => p,
            None => {
                let local = std::path::PathBuf::from(
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Local")),
                )
                .join("Ditto")
                .join("Ditto.db");
                return (0, 0, format!("Ditto 数据库未找到 (已检查 Local 和 Roaming 路径): {}", local.display()));
            }
        };

        // 读取 Ditto 条目
        let (entries, total, err) = crate::database::read_ditto_entries(&ditto_db_path);
        if !err.is_empty() {
            return (0, total, err);
        }

        if entries.is_empty() {
            return (0, 0, "Ditto 数据库中无文本条目".to_string());
        }

        // 导入到本数据库
        let db = self.db.lock().unwrap();
        match db.import_ditto_entries(&entries) {
            Ok(imported) => {
                eprintln!("[import] Ditto: imported {}/{} entries", imported, total);
                (imported, total, String::new())
            }
            Err(e) => (0, total, format!("导入失败: {}", e)),
        }
    }
}
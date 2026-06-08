use rusqlite::{Connection, Result as SqliteResult, params};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use crate::models::{ClipboardItem, ContentType, FavoriteFolder};

pub struct Database {
    conn: Connection,
    images_dir: PathBuf,
}

impl Database {
    pub fn new(db_path: &PathBuf, images_dir: &PathBuf) -> SqliteResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::create_dir_all(images_dir).ok();

        let conn = Connection::open(db_path)?;
        let db = Self {
            conn,
            images_dir: images_dir.clone(),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type    TEXT NOT NULL CHECK(content_type IN ('text', 'image')),
                content_text    TEXT,
                content_path    TEXT,
                content_hash    TEXT NOT NULL UNIQUE,
                original_name   TEXT,
                mime_type       TEXT,
                file_size       INTEGER,
                width           INTEGER,
                height          INTEGER,
                source_ip       TEXT,
                created_at      INTEGER NOT NULL,
                is_favorite     INTEGER NOT NULL DEFAULT 0,
                is_deleted      INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_clipboard_created_at
                ON clipboard_items(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_hash
                ON clipboard_items(content_hash);
            CREATE INDEX IF NOT EXISTS idx_clipboard_type
                ON clipboard_items(content_type);

            CREATE TABLE IF NOT EXISTS config (
                key             TEXT PRIMARY KEY,
                value           TEXT NOT NULL,
                updated_at      INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS favorite_folders (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                sort_order      INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_favorite_folders_order
                ON favorite_folders(sort_order);

            CREATE TABLE IF NOT EXISTS favorite_folder_items (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id       INTEGER NOT NULL,
                item_id         INTEGER NOT NULL,
                added_at        INTEGER NOT NULL,
                sort_order      INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (folder_id) REFERENCES favorite_folders(id) ON DELETE CASCADE,
                FOREIGN KEY (item_id) REFERENCES clipboard_items(id) ON DELETE CASCADE,
                UNIQUE(folder_id, item_id)
            );

            CREATE INDEX IF NOT EXISTS idx_favorite_folder_items_folder
                ON favorite_folder_items(folder_id);
            "#
        )?;
        Ok(())
    }

    pub fn compute_hash(content: &[u8]) -> String {
        let hash = Sha256::digest(content);
        hex::encode(hash)
    }

    pub fn insert_text(&self, text: &str) -> SqliteResult<i64> {
        let hash = Self::compute_hash(text.as_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let existing: Option<i64> = self.conn
            .query_row(
                "SELECT id FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'text'",
                params![hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
            return Ok(id);
        }

        self.conn.execute(
            r#"INSERT INTO clipboard_items
               (content_type, content_text, content_hash, created_at)
               VALUES ('text', ?1, ?2, ?3)"#,
            params![text, hash, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_image(
        &self,
        image_data: &[u8],
        mime_type: &str,
        width: i32,
        height: i32,
    ) -> SqliteResult<(i64, String)> {
        let hash = Self::compute_hash(image_data);

        let existing: Option<(i64, String)> = self.conn
            .query_row(
                "SELECT id, content_path FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'image'",
                params![hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((id, path)) = existing {
            return Ok((id, path));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let ext = match mime_type {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "bin",
        };
        let filename = format!("{}.{}", &hash[..16], ext);

        std::fs::create_dir_all(&self.images_dir).ok();
        let path = self.images_dir.join(&filename);
        // 写入失败则不插入 DB 记录,避免 DB 中出现指向不存在/损坏图片的条目
        std::fs::write(&path, image_data)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("写入图片文件失败 {}: {}", path.display(), e),
            ))))?;

        let file_size = image_data.len() as i64;
        let content_path = format!("images/{}", filename);

        self.conn.execute(
            r#"INSERT INTO clipboard_items
               (content_type, content_path, content_hash, mime_type, file_size, width, height, created_at)
               VALUES ('image', ?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![content_path, hash, mime_type, file_size, width, height, now],
        )?;

        Ok((self.conn.last_insert_rowid(), content_path))
    }

    pub fn get_history(&self, limit: usize, ascending: bool) -> SqliteResult<Vec<ClipboardItem>> {
        let order = if ascending { "ASC" } else { "DESC" };
        let sql = format!(
            r#"SELECT id, content_type, content_text, content_path, content_hash,
                 mime_type, file_size, width, height, source_ip, created_at, is_favorite
             FROM clipboard_items
             WHERE is_deleted = 0
             ORDER BY created_at {}
             LIMIT ?1"#,
            order
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let items = stmt.query_map(params![limit as i64], |row| {
            let content_type_str: String = row.get(1)?;
            let content_type = match content_type_str.as_str() {
                "image" => ContentType::Image,
                _ => ContentType::Text,
            };

            Ok(ClipboardItem {
                id: row.get(0)?,
                content_type,
                content_text: row.get(2)?,
                content_path: row.get(3)?,
                content_hash: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                source_ip: row.get(9)?,
                created_at: row.get(10)?,
                is_favorite: row.get::<_, i32>(11)? != 0,
            })
        })?;

        items.collect()
    }

    /// 搜索历史记录：支持文本内容搜索和图片元数据搜索（尺寸、大小）
    pub fn search_history(&self, query: &str, limit: usize, ascending: bool) -> SqliteResult<Vec<ClipboardItem>> {
        let order = if ascending { "ASC" } else { "DESC" };
        let pattern = format!("%{}%", query);
        
        // 搜索文本内容或图片元数据（尺寸、大小）
        let sql = format!(
            r#"SELECT id, content_type, content_text, content_path, content_hash,
                 mime_type, file_size, width, height, source_ip, created_at, is_favorite
             FROM clipboard_items
             WHERE is_deleted = 0
               AND (
                 content_text LIKE ?1
                 OR CAST(width AS TEXT) || 'x' || CAST(height AS TEXT) LIKE ?1
                 OR CAST(width AS TEXT) LIKE ?1
                 OR CAST(height AS TEXT) LIKE ?1
                 OR CAST(file_size / 1024 AS TEXT) || 'KB' LIKE ?1
                 OR CAST(file_size / 1024 AS TEXT) LIKE ?1
               )
             ORDER BY created_at {}
             LIMIT ?2"#,
            order
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let items = stmt.query_map(params![pattern, limit as i64], |row| {
            let content_type_str: String = row.get(1)?;
            let content_type = match content_type_str.as_str() {
                "image" => ContentType::Image,
                _ => ContentType::Text,
            };

            Ok(ClipboardItem {
                id: row.get(0)?,
                content_type,
                content_text: row.get(2)?,
                content_path: row.get(3)?,
                content_hash: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                source_ip: row.get(9)?,
                created_at: row.get(10)?,
                is_favorite: row.get::<_, i32>(11)? != 0,
            })
        })?;

        items.collect()
    }

    pub fn get_item(&self, id: i64) -> SqliteResult<Option<ClipboardItem>> {
        let item = self.conn.query_row(
            r#"SELECT id, content_type, content_text, content_path, content_hash,
                      mime_type, file_size, width, height, source_ip, created_at, is_favorite
               FROM clipboard_items
               WHERE id = ?1 AND is_deleted = 0"#,
            params![id],
            |row| {
                let content_type_str: String = row.get(1)?;
                let content_type = match content_type_str.as_str() {
                    "image" => ContentType::Image,
                    _ => ContentType::Text,
                };

                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content_type,
                    content_text: row.get(2)?,
                    content_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    mime_type: row.get(5)?,
                    file_size: row.get(6)?,
                    width: row.get(7)?,
                    height: row.get(8)?,
                    source_ip: row.get(9)?,
                    created_at: row.get(10)?,
                    is_favorite: row.get::<_, i32>(11)? != 0,
                })
            },
        ).ok();
        Ok(item)
    }

    pub fn delete(&self, id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_deleted = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn toggle_favorite(&self, id: i64) -> SqliteResult<bool> {
        let current: i32 = self.conn.query_row(
            "SELECT is_favorite FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let new_value = if current == 0 { 1 } else { 0 };
        self.conn.execute(
            "UPDATE clipboard_items SET is_favorite = ?1 WHERE id = ?2",
            params![new_value, id],
        )?;
        Ok(new_value == 1)
    }

    pub fn clear_non_favorites(&self) -> SqliteResult<usize> {
        let mut stmt = self.conn.prepare(
            "SELECT content_path FROM clipboard_items WHERE is_favorite = 0 AND content_type = 'image'"
        )?;
        let paths: Vec<String> = stmt.query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for path in &paths {
            let full_path = self.images_dir.parent().unwrap().join(path);
            let _ = std::fs::remove_file(full_path);
        }

        let count = self.conn.execute(
            "DELETE FROM clipboard_items WHERE is_favorite = 0",
            [],
        )?;
        Ok(count)
    }

    pub fn count(&self) -> SqliteResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn get_config(&self, key: &str) -> SqliteResult<Option<String>> {
        let value: Option<String> = self.conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();
        Ok(value)
    }

    pub fn set_config(&self, key: &str, value: &str) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_images_dir(&self) -> &PathBuf {
        &self.images_dir
    }

    pub fn insert_mock_data(&self, count: usize) -> SqliteResult<usize> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let two_days_ms: i64 = 2 * 24 * 60 * 60 * 1000;
        let mut inserted = 0;

        let current_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;

        for i in 0..count {
            let index = current_count as usize + i + 1;
            let random_offset = (rand_simple(index) % two_days_ms as u64) as i64;
            let created_at = now - random_offset;
            let text = format!("Mock data #{}", index);

            let hash = Self::compute_hash(text.as_bytes());
            
            let existing: Option<i64> = self.conn
                .query_row(
                    "SELECT id FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'text'",
                    params![hash],
                    |row| row.get(0),
                )
                .ok();

            if existing.is_none() {
                self.conn.execute(
                    r#"INSERT INTO clipboard_items
                       (content_type, content_text, content_hash, created_at)
                       VALUES ('text', ?1, ?2, ?3)"#,
                    params![text, hash, created_at],
                )?;
                inserted += 1;
            }
        }

        Ok(inserted)
    }

    // ========== Favorite Folders ==========

    pub fn get_favorite_folders(&self) -> SqliteResult<Vec<FavoriteFolder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, sort_order, created_at FROM favorite_folders ORDER BY sort_order ASC"
        )?;
        let folders = stmt.query_map([], |row| {
            Ok(FavoriteFolder {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(folders)
    }

    pub fn insert_favorite_folder(&self, name: &str) -> SqliteResult<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let max_order: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM favorite_folders",
            [],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO favorite_folders (name, sort_order, created_at) VALUES (?1, ?2, ?3)",
            params![name, max_order + 1, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_favorite_folder(&self, id: i64, name: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE favorite_folders SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_favorite_folder(&self, id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM favorite_folders WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn reorder_favorite_folders(&self, old_index: usize, new_index: usize) -> SqliteResult<()> {
        let folders = self.get_favorite_folders()?;
        if old_index >= folders.len() || new_index > folders.len() {
            return Ok(());
        }

        let mut ids: Vec<i64> = folders.iter().map(|f| f.id).collect();
        let moved_id = ids.remove(old_index);
        let insert_at = if new_index > old_index { new_index - 1 } else { new_index };
        ids.insert(insert_at, moved_id);

        for (i, id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE favorite_folders SET sort_order = ?1 WHERE id = ?2",
                params![i as i32, id],
            )?;
        }
        Ok(())
    }

    pub fn get_favorite_folder_count(&self) -> SqliteResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM favorite_folders",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// 初始化默认收藏夹（仅在数据库为空时创建）
    pub fn init_default_favorite_folders(&self) -> SqliteResult<()> {
        let count = self.get_favorite_folder_count()?;
        if count > 0 {
            return Ok(());
        }
        let defaults = ["工作", "代码片段", "常用链接", "笔记", "阅读清单", "灵感记录", "项目参考", "工具脚本", "API 文档", "设计素材", "临时暂存", "学习资料"];
        for name in &defaults {
            self.insert_favorite_folder(name)?;
        }
        eprintln!("[db] Initialized {} default favorite folders", defaults.len());
        Ok(())
    }

    pub fn add_item_to_favorite_folder(&self, folder_index: usize, item_id: i64) -> SqliteResult<()> {
        // Get folder id by index (based on sort order)
        let folder_id: i64 = self.conn.query_row(
            "SELECT id FROM favorite_folders ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
            params![folder_index as i64],
            |row| row.get(0),
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let max_order: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM favorite_folder_items WHERE folder_id = ?1",
            params![folder_id],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO favorite_folder_items (folder_id, item_id, added_at, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![folder_id, item_id, now, max_order + 1],
        )?;
        Ok(())
    }

    pub fn get_favorite_folder_item_count(&self, folder_id: i64) -> SqliteResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM favorite_folder_items WHERE folder_id = ?1",
            params![folder_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn get_favorite_folder_items(&self, folder_index: usize) -> SqliteResult<Vec<ClipboardItem>> {
        let folder_id: i64 = self.conn.query_row(
            "SELECT id FROM favorite_folders ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
            params![folder_index as i64],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            r#"SELECT ci.id, ci.content_type, ci.content_text, ci.content_path, ci.content_hash,
                 ci.mime_type, ci.file_size, ci.width, ci.height, ci.source_ip, ci.created_at, ci.is_favorite
             FROM clipboard_items ci
             INNER JOIN favorite_folder_items ffi ON ci.id = ffi.item_id
             WHERE ffi.folder_id = ?1 AND ci.is_deleted = 0
             ORDER BY ffi.added_at DESC"#
        )?;

        let items = stmt.query_map(params![folder_id], |row| {
            let content_type_str: String = row.get(1)?;
            let content_type = match content_type_str.as_str() {
                "image" => ContentType::Image,
                _ => ContentType::Text,
            };
            Ok(ClipboardItem {
                id: row.get(0)?,
                content_type,
                content_text: row.get(2)?,
                content_path: row.get(3)?,
                content_hash: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                source_ip: row.get(9)?,
                created_at: row.get(10)?,
                is_favorite: row.get::<_, i32>(11)? != 0,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(items)
    }
}

fn rand_simple(seed: usize) -> u64 {
    let mut state = seed as u64;
    state = state.wrapping_mul(1103515245).wrapping_add(12345);
    state ^= state >> 16;
    state = state.wrapping_mul(1103515245).wrapping_add(12345);
    state
}
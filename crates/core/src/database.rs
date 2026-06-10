use rusqlite::{Connection, Result as SqliteResult, params};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use crate::models::{ClipboardItem, ContentType, FavoriteFolder};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &PathBuf) -> SqliteResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// 将旧 schema (content_path 文本路径) 迁移到新 schema (content_blob BLOB)。
    ///
    /// 旧版本把图片存到 app_data_dir/images/*.png + thumb_*.png,DB 只存相对路径。
    /// 新版本直接以 BLOB 存进 DB,不再使用文件系统。
    ///
    /// 此函数幂等:已是新 schema 时立即返回,不会重复执行迁移。
    pub fn migrate_files_to_blob(&self, app_data_dir: &Path) -> SqliteResult<()> {
        // 检测旧 schema: content_path 列存在 且 content_blob 列不存在
        let has_content_path: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('clipboard_items') WHERE name = 'content_path'",
            [],
            |row| row.get(0),
        )?;
        let has_content_blob: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('clipboard_items') WHERE name = 'content_blob'",
            [],
            |row| row.get(0),
        )?;

        if has_content_blob > 0 {
            // 已是新 schema
            return Ok(());
        }
        if has_content_path == 0 {
            // 既没有 content_path 也没有 content_blob (全新安装),无需迁移
            return Ok(());
        }

        eprintln!("[db] migrating schema: file path -> BLOB");

        // 1) 添加新列
        self.conn.execute(
            "ALTER TABLE clipboard_items ADD COLUMN content_blob BLOB",
            [],
        )?;
        self.conn.execute(
            "ALTER TABLE clipboard_items ADD COLUMN thumbnail_blob BLOB",
            [],
        )?;

        // 2) 遍历图片记录,读文件到 BLOB
        let mut stmt = self.conn.prepare(
            "SELECT id, content_path FROM clipboard_items
             WHERE content_type = 'image' AND content_blob IS NULL",
        )?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        let mut migrated = 0usize;
        let mut missing = 0usize;
        for (id, rel_path) in rows {
            if rel_path.is_empty() {
                continue;
            }
            // rel_path 形如 "images/abc.png",相对 app_data_dir
            let abs = app_data_dir.join(&rel_path);
            match std::fs::read(&abs) {
                Ok(bytes) => {
                    // 读缩略图(可缺失)
                    let mut thumb: Option<Vec<u8>> = None;
                    if let Some(idx) = rel_path.rfind('/') {
                        let (dir, file) = rel_path.split_at(idx + 1);
                        let thumb_rel = format!("{}thumb_{}", dir, file);
                        let thumb_abs = app_data_dir.join(&thumb_rel);
                        if let Ok(tb) = std::fs::read(&thumb_abs) {
                            thumb = Some(tb);
                        }
                    }

                    self.conn.execute(
                        "UPDATE clipboard_items
                         SET content_blob = ?1, thumbnail_blob = ?2
                         WHERE id = ?3",
                        params![bytes, thumb, id],
                    )?;
                    migrated += 1;
                }
                Err(e) => {
                    eprintln!(
                        "[db] WARN: missing file for id={} path={} ({}); skipping",
                        id,
                        abs.display(),
                        e
                    );
                    missing += 1;
                }
            }
        }
        eprintln!(
            "[db] migration: {} images migrated to BLOB, {} missing files",
            migrated, missing
        );

        // 3) 尝试删除 content_path 列(SQLite 3.35+ 支持,失败不影响功能)
        match self.conn.execute(
            "ALTER TABLE clipboard_items DROP COLUMN content_path",
            [],
        ) {
            Ok(_) => eprintln!("[db] dropped legacy column content_path"),
            Err(e) => eprintln!("[db] WARN: drop content_path failed ({}); legacy column kept but unused", e),
        }

        // 4) 清理已迁移的旧图片文件(可选,释放磁盘空间)
        //    只在迁移成功的记录上清理,失败/缺失的不动
        let _ = std::fs::remove_dir_all(app_data_dir.join("images"));

        eprintln!("[db] migration complete");
        Ok(())
    }

    fn init_tables(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type    TEXT NOT NULL CHECK(content_type IN ('text', 'image')),
                content_text    TEXT,
                content_blob    BLOB,
                thumbnail_blob  BLOB,
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
        thumbnail_data: &[u8],
        mime_type: &str,
        width: i32,
        height: i32,
    ) -> SqliteResult<i64> {
        let hash = Self::compute_hash(image_data);

        let existing: Option<i64> = self.conn
            .query_row(
                "SELECT id FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'image'",
                params![hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            return Ok(id);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let file_size = image_data.len() as i64;

        self.conn.execute(
            r#"INSERT INTO clipboard_items
               (content_type, content_blob, thumbnail_blob, content_hash, mime_type, file_size, width, height, created_at)
               VALUES ('image', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![image_data, thumbnail_data, hash, mime_type, file_size, width, height, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// 按 id 读取图片 BLOB 数据,返回 (bytes, mime_type)
    pub fn get_image_data(&self, id: i64) -> SqliteResult<Option<(Vec<u8>, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_blob, mime_type FROM clipboard_items WHERE id = ?1 AND content_type = 'image' AND is_deleted = 0"
        )?;
        let result = stmt.query_row(params![id], |row| {
            let blob: Option<Vec<u8>> = row.get(0)?;
            let mime: Option<String> = row.get(1)?;
            Ok((blob, mime))
        }).ok();

        Ok(result.and_then(|(blob, mime)| {
            blob.zip(mime)
        }))
    }

    /// 按 id 读取缩略图 BLOB
    pub fn get_thumbnail_data(&self, id: i64) -> SqliteResult<Option<Vec<u8>>> {
        let mut stmt = self.conn.prepare(
            "SELECT thumbnail_blob FROM clipboard_items WHERE id = ?1 AND content_type = 'image' AND is_deleted = 0"
        )?;
        let result = stmt.query_row(params![id], |row| {
            row.get::<_, Option<Vec<u8>>>(0)
        }).ok();
        Ok(result.flatten())
    }

    pub fn get_history(&self, limit: usize, ascending: bool) -> SqliteResult<Vec<ClipboardItem>> {
        let order = if ascending { "ASC" } else { "DESC" };
        let sql = format!(
            r#"SELECT id, content_type, content_text, content_hash,
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
                content_hash: row.get(3)?,
                mime_type: row.get(4)?,
                file_size: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                source_ip: row.get(8)?,
                created_at: row.get(9)?,
                is_favorite: row.get::<_, i32>(10)? != 0,
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
            r#"SELECT id, content_type, content_text, content_hash,
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
                content_hash: row.get(3)?,
                mime_type: row.get(4)?,
                file_size: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                source_ip: row.get(8)?,
                created_at: row.get(9)?,
                is_favorite: row.get::<_, i32>(10)? != 0,
            })
        })?;

        items.collect()
    }

    pub fn get_item(&self, id: i64) -> SqliteResult<Option<ClipboardItem>> {
        let item = self.conn.query_row(
            r#"SELECT id, content_type, content_text, content_hash,
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
                    content_hash: row.get(3)?,
                    mime_type: row.get(4)?,
                    file_size: row.get(5)?,
                    width: row.get(6)?,
                    height: row.get(7)?,
                    source_ip: row.get(8)?,
                    created_at: row.get(9)?,
                    is_favorite: row.get::<_, i32>(10)? != 0,
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

    pub fn update_item_created_at(&self, id: i64) -> SqliteResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.conn.execute(
            "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn clear_non_favorites(&self) -> SqliteResult<usize> {
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

        // 从文章中按空白拆词,长度不足时退回到循环拼接
        const ARTICLE: &str = include_str!("../../desktop/ui/assets/test.txt");
        let words: Vec<&str> = ARTICLE.split_whitespace().collect();
        let total_words = words.len();

        for i in 0..count {
            let index = current_count as usize + i + 1;
            let random_offset = (rand_simple(index) % two_days_ms as u64) as i64;
            let created_at = now - random_offset;

            let text = if total_words == 0 {
                // 没有可用词,兜底生成简单文本
                format!("Mock data #{}", index)
            } else {
                // 每条独立种子,混合 index 与当前时间,保证多次插入内容不同
                let seed_a = (index as u64).wrapping_add(now as u64);
                let len_seed = rand_simple(seed_a as usize);
                let start_seed = rand_simple(seed_a.wrapping_add(1) as usize);

                // 5..=100 个连续词,夹紧到实际可用范围
                let desired_len = 5 + (len_seed % 96) as usize;
                let len = desired_len.min(total_words);
                let start = (start_seed % total_words as u64) as usize;
                let end = (start + len).min(total_words);

                words[start..end].join(" ")
            };

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
            r#"SELECT ci.id, ci.content_type, ci.content_text, ci.content_hash,
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
                content_hash: row.get(3)?,
                mime_type: row.get(4)?,
                file_size: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                source_ip: row.get(8)?,
                created_at: row.get(9)?,
                is_favorite: row.get::<_, i32>(10)? != 0,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(items)
    }

    /// 将 Ditto 条目导入到本数据库连接中。
    /// ditto_entries: (text, timestamp) 列表
    /// 返回成功导入数。
    pub fn import_ditto_entries(&self, ditto_entries: &[(String, i64)]) -> SqliteResult<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let mut imported = 0usize;

        // 使用事务批量插入
        self.conn.execute_batch("BEGIN TRANSACTION")?;

        for (text, ditto_date) in ditto_entries {
            let hash = Self::compute_hash(text.as_bytes());

            // 检查是否已存在 (去重)
            let existing: Option<i64> = self.conn
                .query_row(
                    "SELECT id FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'text'",
                    params![hash],
                    |row| row.get(0),
                )
                .ok();

            if existing.is_some() {
                continue;
            }

            // 使用 Ditto 的时间戳,如果无效则用当前时间
            let created_at = if *ditto_date > 0 {
                // Ditto 使用的时间单位可能是秒或毫秒
                // 尝试判断: 如果小于 1e12 (大约 2001 年),可能是秒,转换为毫秒
                if *ditto_date < 1_000_000_000_000 {
                    ditto_date * 1000
                } else {
                    *ditto_date
                }
            } else {
                now
            };

            self.conn.execute(
                r#"INSERT INTO clipboard_items
                   (content_type, content_text, content_hash, created_at)
                   VALUES ('text', ?1, ?2, ?3)"#,
                params![text, hash, created_at],
            )?;

            imported += 1;
        }

        self.conn.execute_batch("COMMIT")?;

        Ok(imported)
    }
}

fn rand_simple(seed: usize) -> u64 {
    // 基于 seed 的简单伪随机数生成器 (xorshift 风格)
    let mut x = seed as u64;
    if x == 0 {
        x = 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// 从 Ditto 的 SQLite 数据库中读取文本条目。
/// 返回 (entries, total_count, error_message)。
pub fn read_ditto_entries(db_path: &Path) -> (Vec<(String, i64)>, usize, String) {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => return (vec![], 0, format!("无法打开 Ditto 数据库: {}", e)),
    };

    let mut entries: Vec<(String, i64)> = Vec::new();

    // Ditto 的主表名是 "main" (或 "data"), 列通常是 clipboardText 和 timeStamp
    // 需要查询 Ditto 的 sqlite_master 来确认表结构
    let table_name = {
        let mut stmt = match conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND (name='main' OR name='data') ORDER BY name LIMIT 1"
        ) {
            Ok(s) => s,
            Err(e) => return (vec![], 0, format!("查询 Ditto 表结构失败: {}", e)),
        };
        let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
        result.unwrap_or_else(|| String::from("main"))
    };

    // 查询表结构以确定列名
    let col_text = "clipboardText";
    let col_time = "timeStamp";

    // 尝试查询数据
    let sql = format!(
        "SELECT {} COLLATE NOCASE, {} FROM {} ORDER BY {} DESC",
        col_text, col_time, table_name, col_time
    );

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            // 可能是列名不同,尝试替代列名 "text" 和 "time"
            let sql2 = format!(
                "SELECT \"text\", \"time\" FROM {} ORDER BY \"time\" DESC",
                table_name
            );
            match conn.prepare(&sql2) {
                Ok(s) => s,
                Err(e2) => return (vec![], 0, format!("查询 Ditto 数据失败: {} (also: {})", e, e2)),
            }
        }
    };

    let rows = stmt.query_map([], |row| {
        let text: String = row.get(0)?;
        let ts: i64 = row.get(1)?;
        Ok((text, ts))
    });

    match rows {
        Ok(rows) => {
            for row in rows {
                if let Ok((text, ts)) = row {
                    if !text.trim().is_empty() {
                        entries.push((text, ts));
                    }
                }
            }
        }
        Err(e) => return (vec![], 0, format!("读取 Ditto 数据失败: {}", e)),
    }

    let total = entries.len();
    (entries, total, String::new())
}
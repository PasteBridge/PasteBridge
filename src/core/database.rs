use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::path::PathBuf;

/// 剪贴板内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Image,
}

/// 剪贴板记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content_type: ContentType,
    pub content_text: Option<String>,
    pub content_path: Option<String>,
    pub content_hash: String,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub source_ip: Option<String>,
    pub created_at: i64,
    pub is_favorite: bool,
}

/// 数据库存储
pub struct Database {
    conn: Connection,
    images_dir: PathBuf,
}

impl Database {
    /// 打开或创建数据库
    pub fn new(db_path: &PathBuf, images_dir: &PathBuf) -> SqliteResult<Self> {
        // 确保目录存在
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

    /// 初始化表结构
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
            "#
        )?;
        Ok(())
    }

    /// 计算内容的 SHA256 哈希
    pub fn compute_hash(content: &[u8]) -> String {
        use sha2::Digest;
        let hash = Sha256::digest(content);
        hex::encode(hash)
    }

    /// 插入文本记录
    pub fn insert_text(&self, text: &str) -> SqliteResult<i64> {
        let hash = Self::compute_hash(text.as_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 检查是否已存在
        let existing: Option<i64> = self.conn
            .query_row(
                "SELECT id FROM clipboard_items WHERE content_hash = ?1 AND content_type = 'text'",
                params![hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            // 更新时间戳
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

    /// 插入图片记录
    pub fn insert_image(
        &self,
        image_data: &[u8],
        mime_type: &str,
        width: i32,
        height: i32,
    ) -> SqliteResult<(i64, String)> {
        let hash = Self::compute_hash(image_data);

        // 检查是否已存在
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

        // 保存图片文件
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
        std::fs::write(&path, image_data).ok();

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

    /// 获取历史记录
    pub fn get_history(&self, limit: usize) -> SqliteResult<Vec<ClipboardItem>> {
         // Limit the amount of text loaded into memory per item by truncating
         // `content_text` to the first 2000 characters. This prevents large
         // clipboard entries from blowing up process memory when building UI
         // models or serializing history.
         let mut stmt = self.conn.prepare(
            r#"SELECT id, content_type, substr(content_text, 1, 200) as content_text, content_path, content_hash,
                 mime_type, file_size, width, height, source_ip, created_at, is_favorite
             FROM clipboard_items
             WHERE is_deleted = 0
             ORDER BY created_at DESC
             LIMIT ?1"#
         )?;

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

    /// 获取单条记录
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

    /// 删除记录（软删除）
    pub fn delete(&self, id: i64) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_deleted = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// 切换收藏状态
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

    /// 清空历史（仅删除非收藏项）
    pub fn clear_non_favorites(&self) -> SqliteResult<usize> {
        // 获取要删除的图片
        let mut stmt = self.conn.prepare(
            "SELECT content_path FROM clipboard_items WHERE is_favorite = 0 AND content_type = 'image'"
        )?;
        let paths: Vec<String> = stmt.query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // 删除文件
        for path in &paths {
            let full_path = self.images_dir.parent().unwrap().join(path);
            let _ = std::fs::remove_file(full_path);
        }

        // 删除记录
        let count = self.conn.execute(
            "DELETE FROM clipboard_items WHERE is_favorite = 0",
            [],
        )?;
        Ok(count)
    }

    /// 获取记录数量
    pub fn count(&self) -> SqliteResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// 获取配置
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

    /// 设置配置
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

    /// 获取数据库路径
    pub fn get_images_dir(&self) -> &PathBuf {
        &self.images_dir
    }
}

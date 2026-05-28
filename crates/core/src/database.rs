use rusqlite::{Connection, Result as SqliteResult, params};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use crate::models::{ClipboardItem, ContentType};

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

    pub fn get_history(&self, limit: usize) -> SqliteResult<Vec<ClipboardItem>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, content_type, content_text, content_path, content_hash,
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
}

fn rand_simple(seed: usize) -> u64 {
    let mut state = seed as u64;
    state = state.wrapping_mul(1103515245).wrapping_add(12345);
    state ^= state >> 16;
    state = state.wrapping_mul(1103515245).wrapping_add(12345);
    state
}
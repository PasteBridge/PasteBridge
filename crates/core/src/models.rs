use serde::{Serialize, Deserialize};

/// 跨平台错误类型。UniFFI 要求 `Result<_, _>` 的 Err 必须是
/// `#[derive(uniffi::Error)]` 标注的具体 enum/class,不能用裸 `String`。
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PasteBridgeError {
    /// 通用错误,消息直接给移动端 toast。
    #[error("{message}")]
    Generic { message: String },
}

impl PasteBridgeError {
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic { message: message.into() }
    }
}

impl From<String> for PasteBridgeError {
    fn from(message: String) -> Self {
        Self::Generic { message }
    }
}

impl From<&str> for PasteBridgeError {
    fn from(message: &str) -> Self {
        Self::Generic { message: message.to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content_type: ContentType,
    pub content_text: Option<String>,
    pub content_hash: String,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub source_ip: Option<String>,
    pub created_at: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteFolder {
    pub id: i64,
    pub name: String,
    pub sort_order: i32,
    pub created_at: i64,
}
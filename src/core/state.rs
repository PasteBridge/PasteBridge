//! Application state shared across modules

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// Shared application state
pub struct AppState {
    /// Clipboard history (newest first)
    pub clipboard_history: Mutex<VecDeque<String>>,
    /// Maximum history size
    pub max_history_size: usize,
    /// Window visibility state
    pub is_window_visible: Mutex<bool>,
}

impl AppState {
    pub fn new(max_history_size: usize) -> Arc<Self> {
        Arc::new(Self {
            clipboard_history: Mutex::new(VecDeque::with_capacity(max_history_size)),
            max_history_size,
            is_window_visible: Mutex::new(false),
        })
    }

    /// Add text to clipboard history
    pub fn push_clipboard(&self, text: String) {
        let mut history = self.clipboard_history.lock().unwrap();
        
        // Skip duplicates
        if history.contains(&text) {
            return;
        }
        
        // Remove oldest if at capacity
        if history.len() >= self.max_history_size {
            history.pop_back();
        }
        
        history.push_front(text);
    }

    /// Get all clipboard history
    pub fn get_history(&self) -> Vec<String> {
        self.clipboard_history.lock().unwrap().iter().cloned().collect()
    }

    /// Set window visibility
    pub fn set_window_visible(&self, visible: bool) {
        *self.is_window_visible.lock().unwrap() = visible;
    }

    /// Check window visibility
    pub fn is_window_visible(&self) -> bool {
        *self.is_window_visible.lock().unwrap()
    }
}

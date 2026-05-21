//! PasteBridge Library
//! 
//! Cross-platform clipboard manager with API server

pub mod core;
pub mod api;
pub mod ui;
pub mod platform;
pub mod tooltip;

mod window_effects;

// Re-export commonly used types
pub use core::state::AppState;

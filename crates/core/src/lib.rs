pub mod models;
pub mod database;
pub mod state;
pub mod device;
pub mod clipboard;
pub mod api;
pub mod discovery;
pub mod sync_device;

pub use state::AppState;

// UniFFI 0.31 纯 proc-macro 模式: 在编译期扫描本 crate 的
// `#[uniffi::export]` / `#[derive(uniffi::Object)]` / `#[derive(uniffi::Record)]` /
// `#[uniffi::export(callback_interface)]` 项,生成内联的 C-ABI shim,供移动端通过
// JNI / Swift 加载 `libpaste_bridge_core` 调用。
// 桌面端不依赖此 shim（直接用 Rust API 即可）。
// 注意:不能与 `include_scaffolding!()` 同 crate 混用。
uniffi::setup_scaffolding!();
slint::include_modules!();

// mimalloc: aggressively returns freed memory to the OS
// Rust's default allocator never unmaps pages, making Task Manager show high memory
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

slint::slint! {
    export component DummyWindow inherits Window {
        width: 1px;
        height: 1px;
        no-frame: true;
        background: transparent;
    }
}

pub mod animation;
pub mod callbacks;
pub mod clipboard;
pub mod drag_out;
pub mod dummy_window;
pub mod focus;
pub mod net;
pub mod popup;
pub mod sync;
pub mod tooltip;
pub mod tray;
pub mod ui;
pub mod platform;
pub mod window_effects;
pub mod window_position;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
use paste_bridge_core::discovery::{DiscoveredPeer, DiscoveryListener};
use slint::{ComponentHandle, Weak};

/// 桌面端 [DiscoveryListener] 适配: 维护去重表 + 把变更推给 Slint UI。
pub struct DesktopDiscoveryListener {
    pub weak: Weak<crate::AppWindow>,
    pub discovered: Arc<std::sync::Mutex<Vec<DiscoveredPeer>>>,
}

impl DiscoveryListener for DesktopDiscoveryListener {
    fn on_discovered(&self, peer: DiscoveredPeer) {
        eprintln!(
            "[mdns] callback: peer={} platform={} addrs={:?} port={} fullname={}",
            peer.device_id, peer.platform, peer.addresses, peer.port, peer.fullname
        );
        let mut list = self.discovered.lock().unwrap();
        if !list.iter().any(|p| p.fullname == peer.fullname) {
            list.push(peer);
            eprintln!("[mdns] new peer added; total={}; pushing to Slint", list.len());
            drop(list);
            push_discovered_to_slint(&self.weak, &self.discovered);
        } else {
            eprintln!("[mdns] duplicate peer, skip");
        }
    }

    fn on_lost(&self, peer: DiscoveredPeer) {
        eprintln!("[mdns] lost peer: {}", peer.fullname);
        let mut list = self.discovered.lock().unwrap();
        list.retain(|p| p.fullname != peer.fullname);
        drop(list);
        push_discovered_to_slint(&self.weak, &self.discovered);
    }
}

fn main() {
    std::env::set_var("SLINT_BACKEND", "winit-skia");
    std::env::set_var("SLINT_STYLE", "material");
    std::env::set_var("ICU4X_DATA_DIR", "");

    // 性能调试：持续刷新以暴露瓶颈，并在每个窗口叠加帧率显示
    // std::env::set_var("SLINT_DEBUG_PERFORMANCE", "refresh_full_speed,overlay");

    // Enable Skia advanced font rendering
    std::env::set_var("SKIA_FONTS_PATH", ""); // Use system fonts

    const WINDOW_WIDTH: f32 = 280.0;
    const WINDOW_HEIGHT: f32 = 396.0;

    eprintln!("Starting PasteBridge...");

    let app_data_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PasteBridge"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let state = paste_bridge_core::state::AppState::new(&app_data_dir, usize::MAX);

    // 初始化默认收藏夹
    state.init_default_favorite_folders();

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    // 从数据库加载持久化设置
    {
        // 加载主题设置
        if let Some(theme) = state.get_config("theme") {
            match theme.as_str() {
                "dark" => app.set_is_dark_mode(true),
                "light" => app.set_is_dark_mode(false),
                _ => {}
            }
            eprintln!("[config] Loaded theme: {}", theme);
        }

        // 加载窗口定位模式
        if let Some(mode) = state.get_config("window-position-mode") {
            if let Ok(mode_val) = mode.parse::<i32>() {
                app.set_window_position_mode(mode_val);
                eprintln!("[config] Loaded window-position-mode: {}", mode_val);
            }
        }

        // 加载焦点恢复设置
        if let Some(restore_focus) = state.get_config("restore-focus") {
            match restore_focus.as_str() {
                "true" | "1" => {
                    focus::set_restore_focus_enabled(true);
                    app.set_restore_focus_enabled(true);
                }
                "false" | "0" => {
                    focus::set_restore_focus_enabled(false);
                    app.set_restore_focus_enabled(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded restore-focus: {}", restore_focus);
        }

        // 加载鼠标悬停聚焦设置
        if let Some(mouse_hover_focus) = state.get_config("mouse-hover-focus") {
            match mouse_hover_focus.as_str() {
                "true" | "1" => {
                    focus::set_mouse_hover_focus_enabled(true);
                    app.set_mouse_hover_focus_enabled(true);
                }
                "false" | "0" => {
                    focus::set_mouse_hover_focus_enabled(false);
                    app.set_mouse_hover_focus_enabled(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded mouse-hover-focus: {}", mouse_hover_focus);
        }

        // 加载置顶（pin）设置
        if let Some(pinned) = state.get_config("pinned") {
            match pinned.as_str() {
                "true" | "1" => {
                    app.set_pinned(true);
                }
                "false" | "0" => {
                    app.set_pinned(false);
                }
                _ => {}
            }
            eprintln!("[config] Loaded pinned: {}", pinned);
        }

        // 加载窗口大小设置(必须在 show() 之前设置,才能影响 Slint 布局系统的 preferred-width/height)
        let mut loaded_width = WINDOW_WIDTH;
        let mut loaded_height = WINDOW_HEIGHT;

        if let Some(width_str) = state.get_config("window-width") {
            if let Ok(width) = width_str.parse::<f32>() {
                if width >= 200.0 && width <= 600.0 {
                    loaded_width = width;
                    eprintln!("[config] Loaded window-width: {}", width);
                }
            }
        }

        if let Some(height_str) = state.get_config("window-height") {
            if let Ok(height) = height_str.parse::<f32>() {
                if height >= 300.0 && height <= 800.0 {
                    loaded_height = height;
                    eprintln!("[config] Loaded window-height: {}", height);
                }
            }
        }

        // 把持久化尺寸设置到 Slint 属性,并调用 set_size 设置 has_explicit_size 标志
        // (必须在 show() 之前调用 set_size,否则 Slint 会用 preferred-width/height 覆盖)
        app.set_initial_width(loaded_width);
        app.set_initial_height(loaded_height);
        app.window().set_size(slint::LogicalSize::new(loaded_width, loaded_height));

        // 枚举本机所有非回环 IPv4 地址,填充到同步面板
        let local_ips = net::list_local_ipv4();
        eprintln!("[net] Detected {} local IPv4 address(es): {:?}", local_ips.len(), local_ips);
        let ips_model = std::rc::Rc::new(slint::VecModel::from(
            local_ips.iter().map(|s| slint::SharedString::from(s.as_str())).collect::<Vec<_>>()
        ));
        app.set_local_ips(ips_model.into());

        // 根据加载的尺寸计算并设置窗口位置
        let pos = window_position::calc_window_position(&app, loaded_width as i32, loaded_height as i32);
        let _ = app.window().set_position(pos);
    }
    let _ = app.window().show();

    tray::IS_VISIBLE.store(true, Ordering::SeqCst);

    focus::start_focus_tracker();

    // Window size change monitor
    {
        let app_weak_for_size = app_weak.clone();
        let state_for_size = state.clone();
        // 初始化为 -1 哨兵值,首次轮询时强制同步当前实际尺寸(避免把默认值当作"无变化"漏保存)
        let last_width = Arc::new(std::sync::atomic::AtomicI32::new(-1));
        let last_height = Arc::new(std::sync::atomic::AtomicI32::new(-1));

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Some(app) = app_weak_for_size.upgrade() {
                    // 窗口隐藏时不保存(避免保存 hide 时的 0x0 或默认值)
                    if !tray::IS_VISIBLE.load(Ordering::SeqCst) {
                        last_width.store(-1, std::sync::atomic::Ordering::Relaxed);
                        last_height.store(-1, std::sync::atomic::Ordering::Relaxed);
                        continue;
                    }

                    let current_size = app.window().size();
                    let current_width = current_size.width as i32;
                    let current_height = current_size.height as i32;

                    // 过滤无效值(窗口最小化时会变成 0)
                    if current_width < 200 || current_height < 300
                        || current_width > 600 || current_height > 800 {
                        continue;
                    }

                    let last_w = last_width.load(std::sync::atomic::Ordering::Relaxed);
                    let last_h = last_height.load(std::sync::atomic::Ordering::Relaxed);

                    if current_width != last_w || current_height != last_h {
                        last_width.store(current_width, std::sync::atomic::Ordering::Relaxed);
                        last_height.store(current_height, std::sync::atomic::Ordering::Relaxed);

                        // 持久化窗口大小
                        state_for_size.set_config("window-width", &current_width.to_string());
                        state_for_size.set_config("window-height", &current_height.to_string());
                        eprintln!("[config] Saved window size: {}x{}", current_width, current_height);

                        // 同步更新 initial-width/initial-height,保持与 width/height 的绑定一致
                        let app_clone = app_weak_for_size.clone();
                        let cw = current_width;
                        let ch = current_height;
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_clone.upgrade() {
                                app.set_initial_width(cw as f32);
                                app.set_initial_height(ch as f32);
                                app.set_size_tooltip_visible(true);
                                app.set_last_width(cw);
                                app.set_last_height(ch);
                            }
                        });
                    }
                } else {
                    break;
                }
            }
        });
    }

    let popup_tooltip = Arc::new(std::sync::Mutex::new(None::<PopupTooltipWindow>));
    let popup_tooltip_clone = popup_tooltip.clone();
    let popup_weak_holder: Arc<std::sync::Mutex<Option<slint::Weak<PopupTooltipWindow>>>> =
        Arc::new(std::sync::Mutex::new(None));
    let popup_weak_holder_clone = popup_weak_holder.clone();

    let clipboard_entries: Arc<std::sync::Mutex<Vec<sync::ClipboardEntry>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let clipboard_entries_clone = clipboard_entries.clone();

    let state_for_init = state.clone();
    let app_for_init = app.as_weak();
    let entries_for_init = clipboard_entries_clone.clone();
    let app_data_dir_for_init = app_data_dir.clone();
    slint::invoke_from_event_loop(move || {
        sync::sync_history_to_ui(&app_for_init, &state_for_init, &entries_for_init, &app_data_dir_for_init, false);
    }).ok();

    let app_weak_clone = app_weak.clone();
    let state_for_clipboard = state.clone();
    let state_for_ui = state.clone();
    let entries_for_update = clipboard_entries_clone.clone();
    clipboard::start_clipboard_monitor(state_for_clipboard, {
        move || {
            let weak = app_weak_clone.clone();
            let state = state_for_ui.clone();
            let entries_for_update = entries_for_update.clone();

            sync::sync_history_to_ui_async(
                weak,
                state,
                entries_for_update,
                std::path::PathBuf::new(),
                true,
            );
        }
    });

    #[cfg(target_os = "windows")]
    window_effects::apply_window_effects();

    #[cfg(target_os = "windows")]
    {
        animation::startup_window_fade_in();
    }

    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);

    let hotkey_id = match manager.register(hotkey) {
        Ok(_) => hotkey.id(),
        Err(e) => {
            eprintln!("Hotkey Ctrl+Alt+V occupied, trying Ctrl+Alt+B... ({})", e);
            let backup_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyB);
            match manager.register(backup_hotkey) {
                Ok(_) => backup_hotkey.id(),
                Err(e2) => {
                    eprintln!("Backup hotkey also failed: {}", e2);
                    eprintln!("A previous instance might be running or hotkeys are used elsewhere. We will continue without a hotkey.");
                    0
                }
            }
        }
    };

    let handles = tray::setup_tray();
    let _tray_icon = handles.tray_icon;
    let weak_for_tray = app_weak.clone();
    tray::start_tray_event_loop(handles.show_id, handles.quit_id, hotkey_id, move || {
        let _ = slint::invoke_from_event_loop({
            let weak = weak_for_tray.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    use slint::ComponentHandle;
                    let is_visible = tray::IS_VISIBLE.load(Ordering::SeqCst);
                    if is_visible {
                        animation::fade_window_out();

                        let _ = app.window().hide();
                        tray::IS_VISIBLE.store(false, Ordering::SeqCst);
                    } else {
                        let cur_w = app.get_initial_width() as i32;
                        let cur_h = app.get_initial_height() as i32;
                        let pos = window_position::calc_window_position(&app, cur_w, cur_h);
                        let _ = app.window().set_position(pos);

                        let _ = app.window().show();
                        tray::IS_VISIBLE.store(true, Ordering::SeqCst);

                        eprintln!("[memory] Tray: Window shown");

                        let hwnd_isize = window_effects::APP_HWND.load(Ordering::SeqCst);
                        if hwnd_isize != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(hwnd_isize as *mut std::ffi::c_void);
                            unsafe {
                                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                            }
                            animation::fade_window_in();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                focus::restore_previous_focus();
                            });
                        }
                    }
                }
            }
        });
    });

    // mDNS 发现的设备列表:Rust 端累计,推到 Slint SharePanel 显示
    // 用 Mutex 保护,browse 回调写 / 回调读 都要抢锁
    let discovered_devices: Arc<std::sync::Mutex<Vec<paste_bridge_core::discovery::DiscoveredPeer>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));

    let callback_ctx = callbacks::CallbackContext {
        app_weak: app_weak.clone(),
        state: state.clone(),
        clipboard_entries: clipboard_entries_clone.clone(),
        popup_tooltip: popup_tooltip_clone.clone(),
        popup_weak_holder: popup_weak_holder_clone.clone(),
        app_data_dir: Arc::new(app_data_dir.clone()),
        discovered_devices: discovered_devices.clone(),
    };
    callbacks::register_all(&app, &callback_ctx);

    let api_state = state.clone();
    std::thread::spawn(move || {
        let server = paste_bridge_core::api::ApiServer::new(18792);
        if let Err(e) = server.start_with_state(api_state) {
            eprintln!("[api] Server error: {}", e);
        }
    });

    // 启动 mDNS 服务发现：注册自身 + 浏览局域网其他 PasteBridge 实例
    match paste_bridge_core::discovery::Discovery::new() {
        Ok(discovery) => {
            let device_id = state.device_id.clone();
            let local_ips = net::list_local_ipv4();

            if let Err(e) = discovery.register(device_id.clone(), "desktop".to_string(), 18792, local_ips.clone()) {
                eprintln!("[mdns] register failed: {}", e);
            }

            let weak_for_discovery = app_weak.clone();
            let discovered_arc = discovered_devices.clone();
            // 适配新 API: 桌面端实现 [DiscoveryListener] trait,把
            // 「去重 + 推 Slint」逻辑放在 on_discovered 里。
            let listener = DesktopDiscoveryListener {
                weak: weak_for_discovery,
                discovered: discovered_arc,
            };
            if let Err(e) = discovery.browse(Box::new(listener)) {
                eprintln!("[mdns] browse failed: {}", e);
            }

            // 让 discovery 句柄常驻到程序退出 (Drop 时自动反注册)
            std::mem::forget(discovery);
        }
        Err(e) => {
            eprintln!("[mdns] init failed: {} (mDNS 不可用)", e);
        }
    }

    tooltip::start_tooltip_zorder_monitor();

    eprintln!("About to run app...");

    popup::create_popup_tooltip(&popup_tooltip, &popup_weak_holder);

    dummy_window::create_and_run();
}

/// 把当前已发现的设备列表同步到 Slint SharePanel。
/// 从 browse 回调和 sync-with-device 回调里调用,必须在 Slint 主事件循环线程里操作。
fn push_discovered_to_slint(
    weak: &slint::Weak<AppWindow>,
    list: &Arc<std::sync::Mutex<Vec<paste_bridge_core::discovery::DiscoveredPeer>>>,
) {
    let weak = weak.clone();
    let list = list.clone();
    // Slint 的所有 UI 操作必须在 event loop 线程里做,直接跨线程 upgrade+set 会失败/卡死
    let _ = slint::invoke_from_event_loop(move || {
        let guard = list.lock().unwrap();
        eprintln!("[mdns-slint] push on event loop: list_len={}", guard.len());
        let ui: Vec<crate::DiscoveredDevice> = guard
            .iter()
            .map(|p| crate::DiscoveredDevice {
                device_id: p.device_id.clone().into(),
                platform: p.platform.clone().into(),
                address: p.addresses.first().cloned().unwrap_or_default().into(),
                port: p.port as i32,
            })
            .collect();
        for d in &ui {
            eprintln!("[mdns-slint]   -> {} {} {}:{}", d.device_id, d.platform, d.address, d.port);
        }
        drop(guard);
        if let Some(app) = weak.upgrade() {
            app.set_discovered_devices(std::rc::Rc::new(slint::VecModel::from(ui)).into());
            eprintln!("[mdns-slint] set_discovered_devices done");
        } else {
            eprintln!("[mdns-slint] weak upgrade failed (app destroyed?)");
        }
    });
}
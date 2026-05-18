use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

pub static IS_VISIBLE: AtomicBool = AtomicBool::new(true);

pub struct TrayHandles {
    pub show_id: String,
    pub quit_id: String,
    pub tray_icon: tray_icon::TrayIcon,
}

pub fn setup_tray() -> TrayHandles {
    use tray_icon::menu::Menu;

    let show_i = MenuItem::new("Show/Hide", true, None);
    let quit_i = MenuItem::new("Quit PasteBridge", true, None);
    let tray_menu = Menu::new();
    tray_menu.append(&show_i).unwrap();
    tray_menu.append(&quit_i).unwrap();

    // 绘制一个灰色的实色图标以防止找不到本地图标报错
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for _ in 0..16*16 {
        rgba.extend_from_slice(&[150, 150, 150, 255]);
    }
    let icon = tray_icon::Icon::from_rgba(rgba, 16, 16).unwrap();

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("PasteBridge")
        .with_icon(icon)
        .build()
        .unwrap();

    TrayHandles {
        show_id: show_i.id().0.clone(),
        quit_id: quit_i.id().0.clone(),
        tray_icon,
    }
}

pub fn start_tray_event_loop<F>(
    show_id: String,
    quit_id: String,
    hotkey_id: u32,
    toggle_window: F,
) where
    F: Fn() + Send + Sync + 'static,
{
    use global_hotkey::{GlobalHotKeyEvent, HotKeyState};

    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        let menu_channel = MenuEvent::receiver();
        
        loop {
            // 处理热键事件
            if let Ok(event) = receiver.try_recv() {
                if event.id == hotkey_id && event.state == HotKeyState::Pressed {
                    toggle_window();
                }
            }

            // 处理托盘菜单事件
            if let Ok(event) = menu_channel.try_recv() {
                if event.id.as_ref() == quit_id {
                    std::process::exit(0);
                } else if event.id.as_ref() == show_id {
                    toggle_window();
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

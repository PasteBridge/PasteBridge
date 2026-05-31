use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem}};
use std::vec::Vec;
use crate::platform::traits::{PlatformTrayTrait, TrayHandles};

pub struct WindowsTray;

impl WindowsTray {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsTray {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTrayTrait for WindowsTray {
    fn setup(&self) -> Result<TrayHandles, String> {
        let show_i = MenuItem::new("Show/Hide", true, None);
        let quit_i = MenuItem::new("Quit PasteBridge", true, None);

        let tray_menu = Menu::new();
        tray_menu.append(&show_i).unwrap();
        tray_menu.append(&quit_i).unwrap();

        let mut rgba = Vec::with_capacity(16 * 16 * 4);
        for _ in 0..16*16 {
            rgba.extend_from_slice(&[150, 150, 150, 255]);
        }
        let icon = tray_icon::Icon::from_rgba(rgba, 16, 16)
            .map_err(|e| e.to_string())?;

        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("PasteBridge")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(TrayHandles {
            show_id: show_i.id().0.clone(),
            quit_id: quit_i.id().0.clone(),
        })
    }

    fn set_tooltip(&self, _tooltip: &str) -> Result<(), String> {
        Ok(())
    }
}
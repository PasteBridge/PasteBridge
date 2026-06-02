use std::sync::{Arc, Mutex};
use slint::ComponentHandle;
use crate::PopupTooltipWindow;

pub fn create_popup_tooltip(
    popup_tooltip: &Arc<Mutex<Option<PopupTooltipWindow>>>,
    popup_weak_holder: &Arc<Mutex<Option<slint::Weak<PopupTooltipWindow>>>>,
) {
    let mut popup_guard = popup_tooltip.lock().unwrap();
    if popup_guard.is_some() {
        return;
    }

    let popup_win = PopupTooltipWindow::new().unwrap();

    popup_win.on_hide_window({
        let popup_win_weak = popup_win.as_weak();
        move || {
            if let Some(p) = popup_win_weak.upgrade() {
                p.hide().unwrap();
            }
        }
    });

    popup_win.on_on_delay_show({
        let popup_win_weak = popup_win.as_weak();
        move || {
            if let Some(popup) = popup_win_weak.upgrade() {
                popup.set_show_state(true);
                popup.show().unwrap();
                crate::tooltip::bring_tooltip_to_front();
            }
        }
    });

    popup_win.window().set_size(slint::LogicalSize::new(200.0, 200.0));
    popup_win.window().set_position(slint::PhysicalPosition::new(-10000, -10000));
    popup_win.hide().unwrap();
    *popup_weak_holder.lock().unwrap() = Some(popup_win.as_weak());
    *popup_guard = Some(popup_win);
    eprintln!("[popup] Tooltip popup window created");
}
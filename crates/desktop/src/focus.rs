#[derive(Debug, Clone, Copy)]
pub struct CaretRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg(target_os = "windows")]
pub fn get_focused_caret_screen_pos() -> Option<CaretRect> {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
            GUITHREADINFO, OBJID_CARET,
        };
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::Accessibility::{AccessibleObjectFromWindow, IAccessible};
        use windows_core::Interface;
        use windows_core::VARIANT;

        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.is_invalid() {
            return None;
        }

        if fg_hwnd.0 as isize == crate::window_effects::APP_HWND.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }

        let get_caret_msaa = |hwnd: windows::Win32::Foundation::HWND| -> Option<CaretRect> {
            let mut acc: *mut core::ffi::c_void = std::ptr::null_mut();
            if AccessibleObjectFromWindow(
                hwnd,
                OBJID_CARET.0 as u32,
                &IAccessible::IID,
                &mut acc,
            ).is_ok() && !acc.is_null() {
                let acc = IAccessible::from_raw(acc);
                let mut left = 0i32;
                let mut top = 0i32;
                let mut width = 0i32;
                let mut height = 0i32;
                let var_child = VARIANT::from(0i32);
                if acc.accLocation(&mut left, &mut top, &mut width, &mut height, &var_child).is_ok() {
                    if width > 0 || height > 0 {
                        return Some(CaretRect {
                            left,
                            top,
                            right: left + width,
                            bottom: top + height,
                        });
                    }
                }
            }
            None
        };

        let caret_from_msaa = get_caret_msaa(fg_hwnd);
        if caret_from_msaa.is_some() {
            return caret_from_msaa;
        }

        let thread_id = GetWindowThreadProcessId(fg_hwnd, None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        if GetGUIThreadInfo(thread_id, &mut info).is_ok() {
            if !info.hwndCaret.is_invalid() {
                let caret_w = info.rcCaret.right - info.rcCaret.left;
                let caret_h = info.rcCaret.bottom - info.rcCaret.top;
                if caret_w >= 0 && caret_h >= 0 {
                    use windows::Win32::Foundation::POINT;
                    let mut pt_tl = POINT { x: info.rcCaret.left, y: info.rcCaret.top };
                    let mut pt_br = POINT { x: info.rcCaret.right, y: info.rcCaret.bottom };
                    if ClientToScreen(info.hwndCaret, &mut pt_tl).as_bool()
                        && ClientToScreen(info.hwndCaret, &mut pt_br).as_bool()
                    {
                        return Some(CaretRect {
                            left: pt_tl.x,
                            top: pt_tl.y,
                            right: pt_br.x,
                            bottom: pt_br.y,
                        });
                    }
                }
            }
        }

        None
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_focused_caret_screen_pos() -> Option<CaretRect> {
    None
}

#[cfg(target_os = "windows")]
static LAST_FOCUS_POS: std::sync::Mutex<Option<CaretRect>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn start_focus_tracker() {
    std::thread::spawn(|| {
        loop {
            if let Some(pos) = get_focused_caret_screen_pos() {
                if let Ok(mut guard) = LAST_FOCUS_POS.lock() {
                    *guard = Some(pos);
                }
            } else {
                if let Ok(mut guard) = LAST_FOCUS_POS.lock() {
                    *guard = None;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_focus_tracker() {}

pub fn get_cached_focus_pos() -> Option<CaretRect> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(guard) = LAST_FOCUS_POS.lock() {
            return *guard;
        }
    }
    None
}
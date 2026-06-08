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
static LAST_FOCUS_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
static RESTORE_FOCUS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[cfg(target_os = "windows")]
pub fn set_restore_focus_enabled(enabled: bool) {
    RESTORE_FOCUS_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    eprintln!("[focus] Restore focus setting changed to: {}", enabled);
}

#[cfg(target_os = "windows")]
pub fn get_restore_focus_enabled() -> bool {
    RESTORE_FOCUS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(target_os = "windows"))]
pub fn set_restore_focus_enabled(_enabled: bool) {}

#[cfg(not(target_os = "windows"))]
pub fn get_restore_focus_enabled() -> bool {
    false
}

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

            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                let fg_hwnd = GetForegroundWindow();
                if !fg_hwnd.is_invalid() {
                    let app_hwnd = crate::window_effects::APP_HWND.load(std::sync::atomic::Ordering::Relaxed);
                    if fg_hwnd.0 as isize != app_hwnd && app_hwnd != 0 {
                        LAST_FOCUS_HWND.store(fg_hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);
                    }
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

#[cfg(target_os = "windows")]
pub fn restore_previous_focus() {
    if !RESTORE_FOCUS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    
    let previous_hwnd = LAST_FOCUS_HWND.load(std::sync::atomic::Ordering::Relaxed);
    if previous_hwnd != 0 {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, GetForegroundWindow};
            use windows::Win32::Foundation::HWND;
            let hwnd = HWND(previous_hwnd as *mut std::ffi::c_void);
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0 as isize != previous_hwnd {
                let _ = SetForegroundWindow(hwnd);
                eprintln!("[focus] Restored focus to previous window: {}", previous_hwnd);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restore_previous_focus() {}

// ── 鼠标悬停聚焦模式：鼠标进入窗口 → 获取焦点，离开 → 归还焦点 ──

#[cfg(target_os = "windows")]
static MOUSE_HOVER_FOCUS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
static MOUSE_HOVER_FOCUS_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
static MOUSE_INSIDE_WINDOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
static HOVER_FOCUS_CAPTURED_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
pub fn set_mouse_hover_focus_enabled(enabled: bool) {
    let was_enabled = MOUSE_HOVER_FOCUS_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
    MOUSE_HOVER_FOCUS_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    eprintln!("[focus] Mouse hover focus setting changed to: {}", enabled);

    if enabled && !was_enabled {
        start_mouse_hover_focus_monitor();
    } else if !enabled && was_enabled {
        MOUSE_HOVER_FOCUS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        // If we currently hold focus, restore it
        if MOUSE_INSIDE_WINDOW.load(std::sync::atomic::Ordering::Relaxed) {
            restore_previous_focus();
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_mouse_hover_focus_enabled(_enabled: bool) {}

#[cfg(target_os = "windows")]
pub fn get_mouse_hover_focus_enabled() -> bool {
    MOUSE_HOVER_FOCUS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(target_os = "windows"))]
pub fn get_mouse_hover_focus_enabled() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn start_mouse_hover_focus_monitor() {
    if MOUSE_HOVER_FOCUS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    MOUSE_HOVER_FOCUS_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(|| {
        eprintln!("[focus] Mouse hover focus monitor started");

        loop {
            if !MOUSE_HOVER_FOCUS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    GetCursorPos, GetWindowRect, GetForegroundWindow, SetForegroundWindow,
                };
                use windows::Win32::Foundation::{POINT, RECT, HWND};

                #[link(name = "user32")]
                extern "system" {
                    fn GetAsyncKeyState(vKey: i32) -> i16;
                }
                const VK_LBUTTON: i32 = 0x01;

                let mut cursor = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut cursor).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                let app_hwnd_val = crate::window_effects::APP_HWND.load(std::sync::atomic::Ordering::Relaxed);
                if app_hwnd_val == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                let app_hwnd = HWND(app_hwnd_val as *mut std::ffi::c_void);
                let mut window_rect = RECT::default();
                if GetWindowRect(app_hwnd, &mut window_rect).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                let inside = cursor.x >= window_rect.left
                    && cursor.x <= window_rect.right
                    && cursor.y >= window_rect.top
                    && cursor.y <= window_rect.bottom;

                // 检测鼠标左键是否按下（正在拖动）
                let left_button_pressed = GetAsyncKeyState(VK_LBUTTON) < 0;

                // 检测鼠标是否在窗口边缘调整大小区域（增大到 20px）
                let resize_border = 20i32;
                let on_resize_edge =
                    (cursor.x >= window_rect.left && cursor.x <= window_rect.left + resize_border)
                    || (cursor.x >= window_rect.right - resize_border && cursor.x <= window_rect.right)
                    || (cursor.y >= window_rect.top && cursor.y <= window_rect.top + resize_border)
                    || (cursor.y >= window_rect.bottom - resize_border && cursor.y <= window_rect.bottom);

                // 如果左键按下且在边缘区域，完全跳过焦点管理（正在调整大小）
                if left_button_pressed && on_resize_edge {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                let was_inside = MOUSE_INSIDE_WINDOW.load(std::sync::atomic::Ordering::Relaxed);

                if inside && !was_inside {
                    // 鼠标进入窗口：保存当前前台窗口，切换焦点到本软件
                    let fg_hwnd = GetForegroundWindow();
                    if !fg_hwnd.is_invalid() && fg_hwnd.0 != app_hwnd.0 {
                        HOVER_FOCUS_CAPTURED_HWND.store(fg_hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);
                        let _ = SetForegroundWindow(app_hwnd);
                        eprintln!("[focus] Mouse entered window, captured focus");
                    }
                } else if !inside && was_inside {
                    // 鼠标离开窗口：归还焦点
                    // 但如果鼠标在调整大小边缘区域，跳过焦点恢复
                    if on_resize_edge {
                        // 不打印日志，避免刷屏
                    } else {
                        let captured = HOVER_FOCUS_CAPTURED_HWND.load(std::sync::atomic::Ordering::Relaxed);
                        if captured != 0 {
                            let captured_hwnd = HWND(captured as *mut std::ffi::c_void);
                            let current_fg = GetForegroundWindow();
                            if !current_fg.is_invalid() && current_fg.0 == app_hwnd.0 {
                                let _ = SetForegroundWindow(captured_hwnd);
                                eprintln!("[focus] Mouse left window, restored focus to: {}", captured);
                            }
                        }
                    }
                } else if inside {
                    // 鼠标仍在窗口内，但检查是否被外部抢走了焦点 → 重新捕获
                    // 但如果鼠标在调整大小边缘区域，跳过
                    if !on_resize_edge {
                        let fg_hwnd = GetForegroundWindow();
                        if !fg_hwnd.is_invalid() && fg_hwnd.0 != app_hwnd.0 {
                            HOVER_FOCUS_CAPTURED_HWND.store(fg_hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);
                            let _ = SetForegroundWindow(app_hwnd);
                            eprintln!("[focus] Focus stolen externally, re-captured");
                        }
                    }
                }

                MOUSE_INSIDE_WINDOW.store(inside, std::sync::atomic::Ordering::Relaxed);
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        eprintln!("[focus] Mouse hover focus monitor stopped");
    });
}
use crate::window_effects::APP_HWND;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static HOVER_TEXT: Mutex<String> = Mutex::new(String::new());

pub static TOOLTIP_HWND: AtomicIsize = AtomicIsize::new(0);
pub static TOOLTIP_VISIBLE: AtomicBool = AtomicBool::new(false);
static TOOLTIP_INIT_STARTED: AtomicBool = AtomicBool::new(false);

pub static HOVER_TOOLTIP_HWND: AtomicIsize = AtomicIsize::new(0);
pub static HOVER_TOOLTIP_VISIBLE: AtomicBool = AtomicBool::new(false);
static HOVER_TOOLTIP_INIT_STARTED: AtomicBool = AtomicBool::new(false);

static HOVER_CALLBACK: Mutex<Option<Box<dyn Fn(i32, i32) -> Option<String> + Send + Sync>>> = Mutex::new(None);

pub fn set_hover_callback<F>(callback: F)
where
    F: Fn(i32, i32) -> Option<String> + Send + Sync + 'static
{
    let mut cb = HOVER_CALLBACK.lock().unwrap();
    *cb = Some(Box::new(callback));
}

pub fn start_hover_detection() {
}

pub fn update_hover_state(y_offset: i32, item_height: i32) {
    let text = {
        let cb = HOVER_CALLBACK.lock().unwrap();
        if let Some(ref callback) = *cb {
            callback(y_offset, item_height)
        } else {
            None
        }
    };

    let mut t = HOVER_TEXT.lock().unwrap();
    *t = text.unwrap_or_default();
}

#[cfg(target_os = "windows")]
pub fn show_hover_tooltip(text: &str) {
    ensure_hover_tooltip_window();
    let hwnd_value = HOVER_TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 {
        return;
    }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = SendMessageW(tooltip_hwnd, WM_SETTEXT, WPARAM(0), LPARAM(text_wide.as_ptr() as isize));

        let tooltip_w = 280;
        let tooltip_h = 250;

        let app_hwnd_val = APP_HWND.load(Ordering::SeqCst);
        if app_hwnd_val == 0 {
            return;
        }

        let app_hwnd = HWND(app_hwnd_val as *mut std::ffi::c_void);
        let mut app_rect = RECT::default();
        if !GetWindowRect(app_hwnd, &mut app_rect).is_ok() {
            return;
        }

        let tooltip_x = app_rect.left - tooltip_w - 5;
        let tooltip_y = app_rect.top + (app_rect.bottom - app_rect.top - tooltip_h) / 2;

        let _ = SetWindowPos(tooltip_hwnd, HWND_TOPMOST, tooltip_x, tooltip_y, tooltip_w, tooltip_h, SWP_NOACTIVATE);

        let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), 245, LWA_ALPHA);
        let _ = ShowWindow(tooltip_hwnd, SW_SHOW);
    }

    HOVER_TOOLTIP_VISIBLE.store(true, Ordering::SeqCst);
}

#[cfg(target_os = "windows")]
pub fn hide_hover_tooltip() {
    if !HOVER_TOOLTIP_VISIBLE.load(Ordering::SeqCst) {
        return;
    }

    let hwnd_value = HOVER_TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 { return; }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        use windows::Win32::Foundation::HWND;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
        let _ = ShowWindow(tooltip_hwnd, SW_HIDE);
    }

    HOVER_TOOLTIP_VISIBLE.store(false, Ordering::SeqCst);
}

#[cfg(target_os = "windows")]
pub fn get_cursor_pos() -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point).is_ok() {
            (point.x, point.y)
        } else {
            (0, 0)
        }
    }
}

#[cfg(target_os = "windows")]
pub fn create_tooltip_window() {
    if TOOLTIP_INIT_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        while APP_HWND.load(Ordering::SeqCst) == 0 {
            thread::sleep(Duration::from_millis(50));
        }

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::*;
            use windows::Win32::Graphics::Gdi::*;

            let class_name: Vec<u16> = "PasteBridgeTooltip\0".encode_utf16().collect();
            let hinstance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into();

            let wcex = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(tooltip_window_proc),
                hInstance: hinstance,
                hCursor: HCURSOR::default(),
                hbrBackground: CreateSolidBrush(COLORREF(0x000000)),
                lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };

            let _ = RegisterClassExW(&wcex);

            let tooltip_hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                windows::core::PCWSTR(class_name.as_ptr()),
                windows::core::PCWSTR::null(),
                WS_POPUP | WS_BORDER,
                CW_USEDEFAULT, CW_USEDEFAULT, 140, 36,
                None, None, hinstance, None,
            );

            if tooltip_hwnd.is_err() {
                eprintln!("[tooltip] Failed to create tooltip window");
                return;
            }

            let tooltip_hwnd = tooltip_hwnd.unwrap();
            TOOLTIP_HWND.store(tooltip_hwnd.0 as isize, Ordering::SeqCst);

            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), 230, LWA_ALPHA);

            eprintln!("[tooltip] Tooltip window created: {:?}", tooltip_hwnd.0);

            std::thread::spawn(move || {
                loop {
                    if TOOLTIP_VISIBLE.load(Ordering::SeqCst) {
                        let mut point = POINT { x: 0, y: 0 };
                        if GetCursorPos(&mut point).is_ok() {
                            let tooltip_w = 140;
                            let tooltip_h = 36;
                            let mut x = point.x - tooltip_w / 2;
                            let mut y = point.y - tooltip_h - 10;

                            let screen_w = GetSystemMetrics(SM_CXSCREEN);
                            let screen_h = GetSystemMetrics(SM_CYSCREEN);

                            if x + tooltip_w > screen_w { x = screen_w - tooltip_w - 10; }
                            if x < 0 { x = 10; }
                            if y + tooltip_h > screen_h { y = point.y - tooltip_h - 20; }
                            if y < 0 { y = 10; }

                            let hwnd = HWND(TOOLTIP_HWND.load(Ordering::SeqCst) as *mut std::ffi::c_void);
                            if hwnd.0 != std::ptr::null_mut() {
                                let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, tooltip_w, tooltip_h, SWP_NOACTIVATE);
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(6));
                }
            });

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(target_os = "windows")]
pub fn create_hover_tooltip_window() {
    if HOVER_TOOLTIP_INIT_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        while APP_HWND.load(Ordering::SeqCst) == 0 {
            thread::sleep(Duration::from_millis(50));
        }

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::*;
            use windows::Win32::Graphics::Gdi::*;

            let class_name: Vec<u16> = "PasteBridgeHoverTooltip\0".encode_utf16().collect();
            let hinstance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into();

            let wcex = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(hover_tooltip_window_proc),
                hInstance: hinstance,
                hCursor: HCURSOR::default(),
                hbrBackground: CreateSolidBrush(COLORREF(0x1a1a1a)),
                lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };

            let _ = RegisterClassExW(&wcex);

            let tooltip_hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                windows::core::PCWSTR(class_name.as_ptr()),
                windows::core::PCWSTR::null(),
                WS_POPUP | WS_BORDER,
                CW_USEDEFAULT, CW_USEDEFAULT, 200, 40,
                None, None, hinstance, None,
            );

            if tooltip_hwnd.is_err() {
                eprintln!("[hover_tooltip] Failed to create tooltip window");
                return;
            }

            let tooltip_hwnd = tooltip_hwnd.unwrap();
            HOVER_TOOLTIP_HWND.store(tooltip_hwnd.0 as isize, Ordering::SeqCst);

            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), 245, LWA_ALPHA);

            eprintln!("[hover_tooltip] Hover tooltip window created: {:?}", tooltip_hwnd.0);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn tooltip_window_proc(hwnd: windows::Win32::Foundation::HWND, msg: u32, wparam: windows::Win32::Foundation::WPARAM, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;

    match msg {
        WM_PAINT => {

            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let brush = CreateSolidBrush(COLORREF(0x000000));
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);

            let mut text_buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut text_buf);
            if len > 0 {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00FFFFFF));

                let font = CreateFontW(
                    -14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, windows::core::w!("Segoe UI"),
                );
                let old_font = SelectObject(hdc, font);

                let mut text_rect = rect;
                text_rect.left = 4;
                text_rect.top = (rect.bottom - rect.top - 14) / 2;
                DrawTextW(hdc, &mut text_buf[..len as usize], &mut text_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                SelectObject(hdc, old_font);
                let _ = DeleteObject(font);
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn hover_tooltip_window_proc(hwnd: windows::Win32::Foundation::HWND, msg: u32, wparam: windows::Win32::Foundation::WPARAM, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;

    match msg {
        WM_PAINT => {

            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let brush = CreateSolidBrush(COLORREF(0x1a1a1a));
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);

            let mut text_buf = [0u16; 4096];
            let len = GetWindowTextW(hwnd, &mut text_buf);
            if len > 0 {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00DDDDDD));

                let font = CreateFontW(
                    -12, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, windows::core::w!("Segoe UI"),
                );
                let old_font = SelectObject(hdc, font);

                let mut text_rect = rect;
                text_rect.left = 8;
                text_rect.right -= 8;
                text_rect.top = 8;
                text_rect.bottom -= 8;

                DrawTextW(hdc, &mut text_buf[..len as usize], &mut text_rect, DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOCLIP);

                SelectObject(hdc, old_font);
                let _ = DeleteObject(font);
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
fn fade_in_tooltip() {
    let hwnd_value = TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 { return; }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        for i in 0..=10 {
            let alpha = (255 * i / 10) as u8;
            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), alpha, LWA_ALPHA);
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn fade_in_hover_tooltip() {
    let hwnd_value = HOVER_TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 { return; }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        for i in 0..=10 {
            let alpha = (255 * i / 10) as u8;
            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), alpha, LWA_ALPHA);
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn fade_out_hover_tooltip() {
    let hwnd_value = HOVER_TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 { return; }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        for i in (0..=8).rev() {
            let alpha = (255 * i / 8) as u8;
            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), alpha, LWA_ALPHA);
            thread::sleep(Duration::from_millis(10));
        }

        let _ = ShowWindow(tooltip_hwnd, SW_HIDE);
    }
}

#[cfg(target_os = "windows")]
fn fade_out_tooltip() {
    let hwnd_value = TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 { return; }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        for i in (0..=5).rev() {
            let alpha = (255 * i / 5) as u8;
            let _ = SetLayeredWindowAttributes(tooltip_hwnd, COLORREF(0), alpha, LWA_ALPHA);
            thread::sleep(Duration::from_millis(20));
        }

        let _ = ShowWindow(tooltip_hwnd, SW_HIDE);
    }
}

#[cfg(target_os = "windows")]
pub fn show_tooltip_at(screen_x: i32, screen_y: i32, text: &str) {
    ensure_tooltip_window();
    let hwnd_value = TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 {
        eprintln!("[tooltip] No tooltip window available");
        return;
    }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = SendMessageW(tooltip_hwnd, WM_SETTEXT, WPARAM(0), LPARAM(text_wide.as_ptr() as isize));

        let tooltip_w = 140;
        let tooltip_h = 36;
        let mut x = screen_x - tooltip_w / 2;
        let mut y = screen_y - tooltip_h - 10;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        if x + tooltip_w > screen_w { x = screen_w - tooltip_w - 10; }
        if x < 0 { x = 10; }
        if y + tooltip_h > screen_h { y = screen_y - tooltip_h - 20; }

        let _ = SetWindowPos(tooltip_hwnd, HWND_TOPMOST, x, y, tooltip_w, tooltip_h, SWP_NOACTIVATE);
        let _ = ShowWindow(tooltip_hwnd, SW_SHOW);

        TOOLTIP_VISIBLE.store(true, Ordering::SeqCst);
        eprintln!("[tooltip] Shown at ({}, {}): {}", x, y, text);

        fade_in_tooltip();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            TOOLTIP_VISIBLE.store(false, Ordering::SeqCst);
            fade_out_tooltip();
            eprintln!("[tooltip] Hidden");
        });
    }
}

#[cfg(target_os = "windows")]
pub fn hide_tooltip() {
    TOOLTIP_VISIBLE.store(false, Ordering::SeqCst);
    fade_out_tooltip();
}

#[cfg(target_os = "windows")]
pub fn show_hover_tooltip_at(window_x: i32, window_y: i32, text: &str) {
    ensure_tooltip_window();
    let hwnd_value = TOOLTIP_HWND.load(Ordering::SeqCst);
    if hwnd_value == 0 {
        eprintln!("[tooltip] No tooltip window available");
        return;
    }

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::*;

        let tooltip_hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = SendMessageW(tooltip_hwnd, WM_SETTEXT, WPARAM(0), LPARAM(text_wide.as_ptr() as isize));

        let text_len = text.len() as i32;
        let tooltip_w = (text_len * 6).min(300).max(100);
        let lines = text.lines().count() as i32;
        let tooltip_h = (lines * 16 + 16).min(200).max(30);

        let tooltip_x = window_x - tooltip_w - 10;
        let tooltip_y = window_y - 10;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        let mut x = tooltip_x;
        let mut y = tooltip_y;
        if x < 10 { x = 10; }
        if y + tooltip_h > screen_h { y = screen_h - tooltip_h - 10; }
        if y < 10 { y = 10; }
        if x > screen_w - 10 { x = window_x + 280 + 10; }

        let _ = SetWindowPos(tooltip_hwnd, HWND_TOPMOST, x, y, tooltip_w, tooltip_h, SWP_NOACTIVATE);

        let _ = SetWindowPos(tooltip_hwnd, HWND_TOPMOST, x, y, tooltip_w, tooltip_h, SWP_NOACTIVATE | SWP_FRAMECHANGED);

        let _ = ShowWindow(tooltip_hwnd, SW_SHOW);

        TOOLTIP_VISIBLE.store(true, Ordering::SeqCst);
        eprintln!("[tooltip] Hover at ({}, {}): {}", x, y, text);

        fade_in_tooltip();
    }
}

#[cfg(target_os = "windows")]
fn ensure_tooltip_window() {
    if TOOLTIP_HWND.load(Ordering::SeqCst) != 0 {
        return;
    }
    create_tooltip_window();
    for _ in 0..20 {
        if TOOLTIP_HWND.load(Ordering::SeqCst) != 0 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "windows")]
fn ensure_hover_tooltip_window() {
    if HOVER_TOOLTIP_HWND.load(Ordering::SeqCst) != 0 {
        return;
    }
    create_hover_tooltip_window();
    for _ in 0..20 {
        if HOVER_TOOLTIP_HWND.load(Ordering::SeqCst) != 0 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
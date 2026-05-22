use std::sync::atomic::{AtomicIsize, AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub static APP_HWND: AtomicIsize = AtomicIsize::new(0);
pub static WINDOW_EFFECTS_READY: AtomicBool = AtomicBool::new(false);
pub static INITIAL_FADE_DONE: AtomicBool = AtomicBool::new(false);

/// 获取鼠标光标位置
pub fn get_cursor_pos() -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        (pt.x, pt.y)
    }
}

pub fn fade_in(hwnd: windows::Win32::Foundation::HWND) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetLayeredWindowAttributes, LWA_ALPHA, GetWindowLongPtrW, SetWindowLongPtrW,
            GWL_EXSTYLE, WS_EX_LAYERED,
        };
        use windows::Win32::Foundation::COLORREF;

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex_style | WS_EX_LAYERED.0 as u32) as isize,
        );

        let steps = 15;
        let delay_ms = 5;
        
        for i in 0..=steps {
            let alpha = (255 * i / steps) as u8;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA).ok();
            thread::sleep(Duration::from_millis(delay_ms));
        }
        
        INITIAL_FADE_DONE.store(true, Ordering::SeqCst);
    }
}

pub fn fade_out(hwnd: windows::Win32::Foundation::HWND) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{SetLayeredWindowAttributes, LWA_ALPHA};
        use windows::Win32::Foundation::COLORREF;

        let steps = 15;
        let delay_ms = 5;
        for i in (0..=steps).rev() {
            let alpha = (255 * i / steps) as u8;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA).ok();
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }
}

struct WinHandle(raw_window_handle::RawWindowHandle);
impl raw_window_handle::HasWindowHandle for WinHandle {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0) })
    }
}

#[cfg(target_os = "windows")]
pub fn apply_window_effects() {
    thread::spawn(move || {
        use std::num::NonZeroIsize;

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
            use windows::Win32::Foundation::HWND;
            use windows::core::PCWSTR;

            let mut hwnd = HWND::default();
            let title: Vec<u16> = "PasteBridge\0".encode_utf16().collect();

            while hwnd.is_invalid() {
                hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())).unwrap_or_default();
                if hwnd.is_invalid() {
                    thread::sleep(Duration::from_millis(100));
                }
            }

            APP_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

            let handle = raw_window_handle::Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).unwrap());
            let raw_handle = raw_window_handle::RawWindowHandle::Win32(handle);

            struct WinHandle(raw_window_handle::RawWindowHandle);
            impl raw_window_handle::HasWindowHandle for WinHandle {
                fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
                    Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0) })
                }
            }

            window_vibrancy::apply_blur(WinHandle(raw_handle), None).ok();
            let _ = window_vibrancy::clear_blur(WinHandle(raw_handle));
            window_vibrancy::apply_blur(WinHandle(raw_handle), Some((0, 0, 0, 0))).ok();

            use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
            use windows::Win32::UI::Controls::MARGINS;

            // 设为 0 避免与 Slint 的 no-frame 冲突，导致显示原生按钮
            let margins = MARGINS {
                cxLeftWidth: 0,
                cxRightWidth: 0,
                cyTopHeight: 0,
                cyBottomHeight: 0,
            };
            DwmExtendFrameIntoClientArea(hwnd, &margins).ok();
            
            // 设置窗口为显示位置但完全透明（为第一次渐变做准备）
            use windows::Win32::UI::WindowsAndMessaging::{SetLayeredWindowAttributes, LWA_ALPHA, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED};
            use windows::Win32::Foundation::COLORREF;
            
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED.0 as u32) as isize);
            
            // 设置完全透明
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA).ok();
            
            // 标记窗口效果已准备好
            WINDOW_EFFECTS_READY.store(true, Ordering::SeqCst);
        }
    });
}

/// 等待窗口效果准备就绪
pub fn wait_for_window_effects_ready() {
    while !WINDOW_EFFECTS_READY.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }
}

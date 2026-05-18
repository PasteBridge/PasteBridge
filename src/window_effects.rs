use std::num::NonZeroIsize;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use std::time::Duration;

pub static APP_HWND: AtomicIsize = AtomicIsize::new(0);

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
            
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            DwmExtendFrameIntoClientArea(hwnd, &margins).ok();
        }
    });
}

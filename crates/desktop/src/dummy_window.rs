use slint::ComponentHandle;
use crate::DummyWindow;

pub fn create_and_run() {
    let dummy_window = DummyWindow::new().unwrap();

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW};
        use windows::Win32::Foundation::HWND;
        use raw_window_handle::RawWindowHandle;
        use raw_window_handle::HasWindowHandle;

        let window = dummy_window.as_weak().unwrap();
        let slint_window = window.window();
        let slint_handle = slint_window.window_handle();
        match slint_handle.window_handle() {
            Ok(raw_win) => {
                let raw = raw_win.as_raw();
                if let RawWindowHandle::Win32(win32_handle) = raw {
                    let hwnd = HWND(win32_handle.hwnd.get() as *mut std::ffi::c_void);
                    unsafe {
                        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_TOOLWINDOW.0 as u32) as isize);
                    }
                }
            }
            Err(e) => {
                eprintln!("[dummy] Could not get window handle (window not realized yet): {:?}", e);
            }
        }
    }

    dummy_window.run().unwrap();
}
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
pub fn apply_window_effects_from_handle(hwnd: windows::Win32::Foundation::HWND) {
    let hwnd_value = hwnd.0 as isize;
    thread::spawn(move || {
        use std::num::NonZeroIsize;

        unsafe {
            let hwnd = windows::Win32::Foundation::HWND(hwnd_value as *mut std::ffi::c_void);
            APP_HWND.store(hwnd_value, Ordering::SeqCst);

            let handle = raw_window_handle::Win32WindowHandle::new(NonZeroIsize::new(hwnd_value).unwrap());
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

// 保留原有的函数，但改进查找逻辑
#[cfg(target_os = "windows")]
pub fn apply_window_effects() {
    thread::spawn(move || {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, EnumWindows, GetWindowTextW, GetClassNameW, WNDENUMPROC};
            use windows::Win32::Foundation::{HWND, BOOL, LPARAM};
            use windows::core::PCWSTR;

            let mut found_hwnd = HWND::default();
            
            // 使用一个全局可变变量来存储找到的窗口句柄
            static mut FOUND_HWND: HWND = HWND(std::ptr::null_mut());
            
            // 使用 EnumWindows 来更精确地查找我们的窗口
            unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
                // 获取窗口标题
                let mut title_buf = [0u16; 256];
                let title_len = GetWindowTextW(hwnd, &mut title_buf);
                let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
                
                // 获取窗口类名
                let mut class_buf = [0u16; 256];
                let class_len = GetClassNameW(hwnd, &mut class_buf);
                let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
                
                // 检查窗口是否符合我们的条件
                // 我们不仅检查标题，还检查类名，因为文件资源管理器的类名通常是 CabinetWClass 或类似的
                if title == "PasteBridge" && !class_name.contains("Cabinet") && !class_name.contains("Explore") {
                    FOUND_HWND = hwnd;
                    return BOOL(0); // 停止枚举
                }
                
                BOOL(1) // 继续枚举
            }

            while found_hwnd.is_invalid() {
                // 首先重置全局变量
                FOUND_HWND = HWND::default();
                
                // 尝试 FindWindowW 作为快速路径
                let title: Vec<u16> = "PasteBridge\0".encode_utf16().collect();
                found_hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())).unwrap_or_default();
                
                // 验证找到的窗口类名
                if !found_hwnd.is_invalid() {
                    let mut class_buf = [0u16; 256];
                    let class_len = GetClassNameW(found_hwnd, &mut class_buf);
                    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
                    
                    // 如果类名表明是文件资源管理器，我们就忽略这个窗口
                    if class_name.contains("Cabinet") || class_name.contains("Explore") {
                        found_hwnd = HWND::default();
                    }
                }
                
                if found_hwnd.is_invalid() {
                    // 如果快速路径失败或被拒绝，使用 EnumWindows 更精确地查找
                    EnumWindows(Some(enum_windows_proc), LPARAM(0));
                    found_hwnd = FOUND_HWND;
                }
                
                if found_hwnd.is_invalid() {
                    thread::sleep(Duration::from_millis(100));
                }
            }

            apply_window_effects_from_handle(found_hwnd);
        }
    });
}

/// 等待窗口效果准备就绪
pub fn wait_for_window_effects_ready() {
    while !WINDOW_EFFECTS_READY.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }
}

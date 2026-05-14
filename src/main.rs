slint::include_modules!();

use std::sync::Mutex;
use std::collections::VecDeque;

struct ClipboardHistory {
    items: VecDeque<String>,
    max_size: usize,
}

impl ClipboardHistory {
    fn new(max_size: usize) -> Self {
        Self {
            items: VecDeque::new(),
            max_size,
        }
    }

    fn push(&mut self, text: String) {
        // Avoid duplicates
        if self.items.contains(&text) {
            return;
        }
        if self.items.len() >= self.max_size {
            self.items.pop_back();
        }
        self.items.push_front(text);
    }
}

static CLIPBOARD_HISTORY: Mutex<Option<ClipboardHistory>> = Mutex::new(None);

#[cfg(target_os = "windows")]
mod window_drag {
    use std::os::raw::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn ReleaseCapture() -> i32;
        fn SendMessageW(hwnd: *mut c_void, msg: u32, wparam: u32, lparam: isize) -> isize;
        fn GetForegroundWindow() -> *mut c_void;
    }

    pub fn start_drag() {
        unsafe {
            let hwnd = GetForegroundWindow();
            ReleaseCapture();
            SendMessageW(hwnd, 0x112, 0xF012, 0);
            // 收到这个返回后，代表系统拖拽完成，此时鼠标必定已经松开。
            // 补发一个 WM_LBUTTONUP (0x0202) 事件给应用，防止后端丢失 MouseUp 状态而造成事件无限捕获。
            SendMessageW(hwnd, 0x0202, 0, 0);
        }
    }
}

fn main() {
    // 强制使用支持透明度的软件渲染或特定后端来确保窗口真正的透明
    std::env::set_var("SLINT_BACKEND", "winit-software");
    std::env::set_var("SLINT_STYLE", "fluent"); // 使用 fluent 风格

    eprintln!("Starting PasteBridge...");

    // Initialize clipboard history
    let mut init_history = ClipboardHistory::new(20);
    init_history.push("Mock Data 1: Hello World!".to_string());
    init_history.push("Mock Data 2: Rust is awesome!".to_string());
    init_history.push("Mock Data 3: Slint UI test...".to_string());
    *CLIPBOARD_HISTORY.lock().unwrap() = Some(init_history);

    let app = AppWindow::new().unwrap();
    let app_weak = app.as_weak();

    // Set up clipboard monitoring in background - only updates history
    std::thread::spawn(move || {
        eprintln!("Clipboard monitoring thread started");
        use arboard::Clipboard;
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create clipboard: {}", e);
                return;
            }
        };
        let mut last_content = String::new();
        eprintln!("Clipboard created successfully");

        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            if let Ok(content) = clipboard.get_text() {
                if !content.is_empty() && content != last_content {
                    last_content = content.clone();
                    eprintln!("New clipboard content: {}", &content[..content.len().min(50)]);

                    // Add to history
                    if let Some(ref mut history) = *CLIPBOARD_HISTORY.lock().unwrap() {
                        history.push(content.clone());
                    }
                }
            }
        }
    });

    // Use Slint Timer to update UI from main thread
    eprintln!("DEBUG: About to create Timer");
    let weak_clip_ui = app_weak.clone();
    let _timer = slint::Timer::default();
    _timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
        eprintln!("DEBUG: Timer callback fired!");
        if let Some(w) = weak_clip_ui.upgrade() {
            let history = CLIPBOARD_HISTORY.lock().unwrap();
            if let Some(ref h) = *history {
                let items: Vec<slint::SharedString> = h.items.iter().map(|s| s.clone().into()).collect();
                eprintln!("Timer: updating UI with {} items", items.len());
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                w.set_clipboard_history(model.into());
            } else {
                eprintln!("Timer: history is None!");
            }
        } else {
            eprintln!("Timer: window upgrade failed!");
        }
    });
    eprintln!("DEBUG: Timer created and started");

    //Windows
    #[cfg(target_os = "windows")]
    {
        use std::num::NonZeroIsize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
                use windows::Win32::Foundation::HWND;
                use windows::core::PCWSTR;
                
                let mut hwnd = HWND::default();
                let title: Vec<u16> = "PasteBridge\0".encode_utf16().collect();
                
                while hwnd.is_invalid() {
                    hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())).unwrap_or_default();
                    if hwnd.is_invalid() {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                
                let handle = raw_window_handle::Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).unwrap());
                let raw_handle = raw_window_handle::RawWindowHandle::Win32(handle);
                
                struct WinHandle(raw_window_handle::RawWindowHandle);
                impl raw_window_handle::HasWindowHandle for WinHandle {
                    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
                        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0) })
                    }
                }
                
                window_vibrancy::apply_acrylic(WinHandle(raw_handle), None).ok();
                // 尝试添加 clear background，有时候毛玻璃需要清除DWM背景或者Slint渲染器配置
                let _ = window_vibrancy::clear_blur(WinHandle(raw_handle)); // clearing in case of conflict
                window_vibrancy::apply_blur(WinHandle(raw_handle), Some((0, 0, 0, 0))).ok();
            }
        });
    }

    // Setup callbacks using WeakWindow
    let weak1 = app_weak.clone();
    app.on_minimize_window(move || {
        if let Some(w) = weak1.upgrade() {
            w.window().hide().ok();
        }
    });

    let weak2 = app_weak.clone();
    app.on_hide_window(move || {
        eprintln!("Hide window callback called!");
        eprintln!("About to call process::exit(0)");
        std::process::exit(0);
    });

    let weak4 = app_weak.clone();
    app.on_start_drag(move || {
        if let Some(w) = weak4.upgrade() {
            #[cfg(target_os = "windows")]
            {
                w.window().show().ok();
                window_drag::start_drag();
            }
        }
    });

    // Clipboard entry callback - not used anymore, UI updates via Timer
    app.on_new_clipboard_entry(move |_text| {
        // UI update is now handled by Timer
    });

    // Global hotkey
    std::thread::spawn(move || {
        use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent};
        use global_hotkey::hotkey::{HotKey, Modifiers, Code};

        let manager = GlobalHotKeyManager::new().unwrap();
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);

        manager.register(hotkey).ok();

        let receiver = GlobalHotKeyEvent::receiver();
        let weak3 = app_weak.clone();

        loop {
            if let Ok(event) = receiver.recv() {
                if event.state == global_hotkey::HotKeyState::Pressed {
                    if let Some(window) = weak3.upgrade() {
                        let visible = window.window().is_visible();
                        if visible {
                            window.window().hide().ok();
                        } else {
                            window.window().show().ok();
                        }
                    }
                }
            }
        }
    });

    eprintln!("About to run app...");
    app.run().ok();
    eprintln!("App run() finished");
}

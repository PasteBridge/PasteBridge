use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use crate::AppWindow;

pub struct ClipboardHistory {
    pub items: VecDeque<String>,
    pub max_size: usize,
}

impl ClipboardHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: VecDeque::new(),
            max_size,
        }
    }

    pub fn push(&mut self, text: String) {
        if self.items.contains(&text) { return; }
        if self.items.len() >= self.max_size {
            self.items.pop_back();
        }
        self.items.push_front(text);
    }
}

/// 将文本写入系统剪贴板（text 必须是_owned 的，即 clone 或 String）
pub fn set_clipboard_text(text: String) {
    thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    });
}

pub fn start_clipboard_monitor(
    clipboard_weak: slint::Weak<AppWindow>,
    max_size: usize,
) {
    thread::spawn(move || {
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
        if let Ok(content) = clipboard.get_text() {
            last_content = content;
        }
        eprintln!("Clipboard created successfully");

        let mut history = ClipboardHistory::new(max_size);

        loop {
            thread::sleep(Duration::from_millis(500));

            if let Ok(content) = clipboard.get_text() {
                if !content.is_empty() && content != last_content {
                    last_content = content.clone();
                    let preview: String = content.chars().take(50).collect();
                    eprintln!("New clipboard content: {}", preview);

                    history.push(content.clone());
                    let items: Vec<slint::SharedString> = history.items.iter().map(|s| s.clone().into()).collect();
                    let weak = clipboard_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak.upgrade() {
                            let model = std::rc::Rc::new(slint::VecModel::from(items));
                            w.set_clipboard_history(model.into());
                        }
                    });
                }
            }
        }
    });
}

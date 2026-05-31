pub fn on_copy_item(text: String) {
    eprintln!("[ui:callback] copy_item called: {}",
        text.chars().take(20).collect::<String>());

    paste_bridge_core::clipboard::set_clipboard_text(text);
}

pub fn on_hide_window() {
    eprintln!("[ui:callback] hide_window called");
}

pub fn on_minimize_window() {
    eprintln!("[ui:callback] minimize_window called");
}
use std::sync::Arc;
use slint::ComponentHandle;
use crate::AppWindow;

#[derive(Clone)]
pub struct ClipboardEntry {
    pub text: slint::SharedString,
    pub id: i64,
}

pub fn truncate_for_display(text: &str, max_bytes: usize) -> slint::SharedString {
    if text.len() <= max_bytes {
        return text.into();
    }

    let mut cut_point = max_bytes;
    while cut_point > 0 && !text.is_char_boundary(cut_point) {
        cut_point -= 1;
    }

    let truncated = &text[..cut_point];

    if let Some(last_nl) = truncated.rfind('\n') {
        format!("{}\n...", &truncated[..last_nl]).into()
    } else {
        format!("{}...", truncated).into()
    }
}

pub fn sync_history_to_ui(
    weak: &slint::Weak<AppWindow>,
    state: &Arc<paste_bridge_core::state::AppState>,
    entries_lock: &Arc<std::sync::Mutex<Vec<ClipboardEntry>>>,
    trigger_animation: bool,
) {
    if let Some(w) = weak.upgrade() {
        let ascending = w.get_sort_ascending();
        let history = state.get_history(ascending);
        let entries: Vec<ClipboardEntry> = history.iter()
            .filter_map(|item| {
                item.content_text.clone().map(|text| ClipboardEntry {
                    text: text.into(),
                    id: item.id,
                })
            })
            .collect();

        let items: Vec<slint::SharedString> = entries.iter()
            .map(|e| truncate_for_display(&e.text, 2000))
            .collect();

        {
            let mut lock = entries_lock.lock().unwrap();
            *lock = entries;
        }

        let model = std::rc::Rc::new(slint::VecModel::from(items));
        w.set_clipboard_history(model.into());

        if trigger_animation {
            crate::animation::trigger_content_update_fade(w.as_weak());
        }
    }
}
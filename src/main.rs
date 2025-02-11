use cursive::views::{SelectView, TextArea};
use cursive::{event::Event, traits::*};
use std::sync::{Arc, Mutex};

/// Enum representing the available text transformation choices.
#[derive(Clone, Copy, Debug)]
enum Choice {
    Upper,
    Lower,
    Cap,
}

/// The `Editor` struct now holds:
/// - `selection`: the current highlighted text (if any)
/// - `selection_start` and `selection_end`: byte indices for the current selection
/// - `original_selection_start` and `original_selection_end`: the original boundaries when the selection was first created
#[derive(Clone)]
struct Editor {
    selection: String,
    selection_start: usize,
    selection_end: usize,
    original_selection_start: usize,
    original_selection_end: usize,
}

impl Editor {
    /// Creates a new editor with no selection.
    fn new() -> Self {
        Self {
            selection: String::new(),
            selection_start: 0,
            selection_end: 0,
            original_selection_start: 0,
            original_selection_end: 0,
        }
    }

    /// Updates the editor’s selection state.
    ///
    /// If `selection_start` equals `selection_end`, the selection is cleared.
    /// Otherwise, the method extracts the substring from `content` between these
    /// indices and sets it as the current selection.
    fn update_selection(&mut self, content: String, selection_start: usize, selection_end: usize) {
        if selection_start == selection_end {
            self.selection.clear();
        } else {
            let sel = &content[selection_start..selection_end];
            self.selection = sel.to_string();
        }
        self.selection_start = selection_start;
        self.selection_end = selection_end;
    }

    /// Runs the editor inside a Cursive text UI.
    fn run(self) {
        // Use an Arc<Mutex<Editor>> for shared, mutable, thread-safe state.
        let editor = Arc::new(Mutex::new(self));
        let mut siv = cursive::default();

        // Create a full-screen text area named "main".
        let main_text_area = TextArea::new().with_name("main").full_screen();
        siv.add_layer(main_text_area);

        // -------------------------------------------------
        // Cursor Movement Callbacks (WASD controls)
        // -------------------------------------------------

        siv.add_global_callback(Event::CtrlChar('d'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                if cur < content.len() {
                    if let Some(next_char) = content[cur..].chars().next() {
                        view.set_cursor(cur + next_char.len_utf8());
                    }
                }
            });
        });

        siv.add_global_callback(Event::CtrlChar('a'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                if cur > 0 {
                    let new_cursor = content[..cur]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    view.set_cursor(new_cursor);
                }
            });
        });

        siv.add_global_callback(Event::CtrlChar('s'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                let col = content[current_line_start..cur].chars().count();
                let current_line_end = content[cur..]
                    .find('\n')
                    .map(|pos| cur + pos)
                    .unwrap_or(content.len());
                if current_line_end < content.len() {
                    let next_line_start = current_line_end + 1;
                    let next_line_end = content[next_line_start..]
                        .find('\n')
                        .map(|pos| next_line_start + pos)
                        .unwrap_or(content.len());
                    let next_line_length = content[next_line_start..next_line_end].chars().count();
                    let new_col = col.min(next_line_length);
                    let mut byte_offset = next_line_start;
                    for (i, (b_index, _)) in content[next_line_start..].char_indices().enumerate() {
                        if i == new_col {
                            byte_offset = next_line_start + b_index;
                            break;
                        }
                    }
                    view.set_cursor(byte_offset);
                }
            });
        });

        siv.add_global_callback(Event::CtrlChar('w'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                let col = content[current_line_start..cur].chars().count();
                if current_line_start > 0 {
                    let prev_line_start = content[..current_line_start - 1]
                        .rfind('\n')
                        .map(|pos| pos + 1)
                        .unwrap_or(0);
                    let prev_line_length = content[prev_line_start..current_line_start - 1]
                        .chars()
                        .count();
                    let new_col = col.min(prev_line_length);
                    let mut byte_offset = prev_line_start;
                    for (i, (b_index, _)) in content[prev_line_start..].char_indices().enumerate() {
                        if i == new_col {
                            byte_offset = prev_line_start + b_index;
                            break;
                        }
                    }
                    view.set_cursor(byte_offset);
                }
            });
        });

        // -------------------------------------------------
        // Custom Selection Expansion with Ctrl+p
        // -------------------------------------------------
        {
            let editor = editor.clone();
            siv.add_global_callback(Event::CtrlChar('p'), move |s| {
                s.call_on_name("main", |view: &mut TextArea| {
                    let content = view.get_content();
                    let content_str = content.to_string();
                    // Remove any existing capture delimiters.
                    let cleaned_content = content_str.replace("<|", "").replace("|>", "");

                    // Get current selection boundaries from shared state.
                    let (selection_start, selection_end) = {
                        let ed = editor.lock().unwrap();
                        (ed.selection_start, ed.selection_end)
                    };

                    // Expand left: search backwards in the cleaned text for a space.
                    let new_bound_l = if selection_start > 0 {
                        cleaned_content[..selection_start]
                            .rfind(' ')
                            .map(|pos| pos + 1)
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    // Expand right: search forwards for a space.
                    let new_bound_r = match cleaned_content[selection_end..].find(' ') {
                        Some(pos) => selection_end + pos,
                        None => cleaned_content.len(),
                    };

                    // Update the editor state with the cleaned text and new boundaries.
                    let mut ed = editor.lock().unwrap();
                    ed.update_selection(cleaned_content.clone(), new_bound_l, new_bound_r);

                    // Update the view: insert delimiters for display.
                    let new_content = format!(
                        "{}<|{}|>{}",
                        &cleaned_content[..new_bound_l],
                        ed.selection,
                        &cleaned_content[new_bound_r..]
                    );
                    view.set_content(new_content);
                });
            });
        }

        // -------------------------------------------------
        // Toggle Selection with Ctrl+Space
        // -------------------------------------------------
        {
            let editor = editor.clone();
            siv.add_global_callback(Event::CtrlChar(' '), move |s| {
                s.call_on_name("main", |view: &mut TextArea| {
                    let orig_cursor = view.cursor();
                    let content = view.get_content();
                    let mut ed = editor.lock().unwrap();
                    if ed.selection.is_empty() {
                        // When no selection is active, select the character at the cursor.
                        if orig_cursor < content.len() {
                            if let Some(ch) = content[orig_cursor..].chars().next() {
                                let char_len = ch.len_utf8();
                                let end = orig_cursor + char_len;
                                // Update current selection and also record the original boundaries.
                                ed.update_selection(content.to_string(), orig_cursor, end);
                                ed.original_selection_start = orig_cursor;
                                ed.original_selection_end = end;
                                let new_content = format!(
                                    "{}<|{}|>{}",
                                    &content[..orig_cursor],
                                    ed.selection,
                                    &content[end..]
                                );
                                view.set_content(new_content);
                                view.set_cursor(orig_cursor + 2);
                            }
                        }
                    } else {
                        // Remove the inserted markers and clear the selection.
                        let marker = format!("<|{}|>", ed.selection);
                        let new_content = content.replace(&marker, &ed.selection);
                        view.set_content(new_content);
                        ed.selection.clear();
                        ed.selection_start = orig_cursor;
                        ed.selection_end = orig_cursor;
                        view.set_cursor(orig_cursor.saturating_sub(2));
                    }
                });
            });
        }

        // -------------------------------------------------
        // Reduce Selection with Ctrl+n
        // -------------------------------------------------
        {
            let editor = editor.clone();
            siv.add_global_callback(Event::CtrlChar('n'), move |s| {
                s.call_on_name("main", |view: &mut TextArea| {
                    // First remove any markers from the view.
                    let content = view.get_content();
                    let cleaned_content = content.replace("<|", "").replace("|>", "");

                    // Retrieve the original selection boundaries.
                    let (orig_start, orig_end) = {
                        let ed = editor.lock().unwrap();
                        (ed.original_selection_start, ed.original_selection_end)
                    };

                    // Update the internal selection back to the original boundaries.
                    {
                        let mut ed = editor.lock().unwrap();
                        ed.update_selection(cleaned_content.clone(), orig_start, orig_end);
                    }

                    // Update the view with the original selection reinserted.
                    let new_content = format!(
                        "{}<|{}|>{}",
                        &cleaned_content[..orig_start],
                        &cleaned_content[orig_start..orig_end],
                        &cleaned_content[orig_end..]
                    );
                    view.set_content(new_content);
                    // Optionally, reset the cursor to the end of the original selection.
                    view.set_cursor(orig_start + 2);
                });
            });
        }

        // -------------------------------------------------
        // Transformation Menu with Ctrl+u
        // -------------------------------------------------
        siv.add_global_callback(Event::CtrlChar('u'), |s| {
            let mut sv: SelectView<Choice> = SelectView::new();
            sv.add_item("Uppercase", Choice::Upper);
            sv.add_item("Lowercase", Choice::Lower);
            sv.add_item("Capitalized", Choice::Cap);

            sv.set_on_submit(|s, item| {
                s.call_on_name("main", |view: &mut TextArea| {
                    let content = view.get_content();
                    let new_content = match item {
                        Choice::Upper => content.to_uppercase(),
                        Choice::Lower => content.to_lowercase(),
                        Choice::Cap => capitalize(&content),
                    };
                    view.set_content(new_content);
                });
                s.pop_layer();
            });
            s.add_layer(sv);
        });

        siv.run();
    }
}

/// Capitalizes each word in the provided text.
/// For example, "hello world" becomes "Hello World".
fn capitalize(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn main() {
    let editor = Editor::new();
    editor.run();
}

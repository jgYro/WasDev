use cursive::views::{SelectView, TextArea};
use cursive::{event::Event, traits::*};

/// Enum representing the available text transformation choices.
#[derive(Clone, Copy, Debug)]
enum Choice {
    Upper,
    Lower,
    Cap,
}

/// The `Editor` struct holds our editor's state:
/// - `selection`: the currently highlighted text (if any)
/// - `selection_start` and `selection_end`: byte indices of the selection within the text
#[derive(Clone)]
struct Editor {
    selection: String,
    selection_start: usize,
    selection_end: usize,
}

impl Editor {
    /// Creates a new editor with no selection.
    fn new() -> Self {
        Self {
            selection: String::new(),
            selection_start: 0,
            selection_end: 0,
        }
    }

    /// Updates the editor’s selection state.
    ///
    /// If `selection_start` equals `selection_end`, the selection is cleared.
    /// Otherwise, the method extracts the substring from `content` between these
    /// indices and sets it as the current selection.
    fn update_selection(&mut self, content: String, selection_start: usize, selection_end: usize) {
        if selection_start == selection_end {
            // No range specified: clear the selection.
            self.selection.clear();
        } else {
            // Slice the content to obtain the selection.
            let update_selection = &content[selection_start..selection_end];
            self.selection = update_selection.to_string();
        }
        self.selection_start = selection_start;
        self.selection_end = selection_end;
    }

    /// Runs the editor inside a Cursive text UI.
    fn run(mut self) {
        // Initialize the Cursive TUI.
        let mut siv = cursive::default();

        // Create a full-screen text area named "main".
        let main_text_area = TextArea::new().with_name("main").full_screen();
        siv.add_layer(main_text_area);

        // -------------------------------------------------
        // Cursor Movement Callbacks (WASD controls)
        // -------------------------------------------------

        // Move the cursor right when Ctrl+d is pressed.
        siv.add_global_callback(Event::CtrlChar('d'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                // Do nothing if already at the end.
                if cur >= content.len() {
                    return;
                }
                // Get the next character and advance the cursor by its byte length.
                if let Some(next_char) = content[cur..].chars().next() {
                    let new_cursor = cur + next_char.len_utf8();
                    view.set_cursor(new_cursor);
                }
            });
        });

        // Move the cursor left when Ctrl+a is pressed.
        siv.add_global_callback(Event::CtrlChar('a'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                // Do nothing if already at the beginning.
                if cur == 0 {
                    return;
                }
                // Find the beginning of the previous character.
                let new_cursor = content[..cur]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                view.set_cursor(new_cursor);
            });
        });

        // Move the cursor down when Ctrl+s is pressed.
        siv.add_global_callback(Event::CtrlChar('s'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();

                // Determine the start of the current line.
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                // Calculate the current column (number of characters from the line start).
                let col = content[current_line_start..cur].chars().count();

                // Find the end of the current line.
                let current_line_end = content[cur..]
                    .find('\n')
                    .map(|pos| cur + pos)
                    .unwrap_or(content.len());

                // If there is no next line, do nothing.
                if current_line_end >= content.len() {
                    return;
                }

                // The next line starts immediately after the newline character.
                let next_line_start = current_line_end + 1;
                // Determine the end of the next line.
                let next_line_end = content[next_line_start..]
                    .find('\n')
                    .map(|pos| next_line_start + pos)
                    .unwrap_or(content.len());

                // Count the number of characters in the next line.
                let next_line_length = content[next_line_start..next_line_end].chars().count();
                // The target column is the same as the current one unless the next line is shorter.
                let new_col = col.min(next_line_length);

                // Convert the target column (character count) to a byte offset.
                let mut byte_offset = next_line_start;
                for (i, (b_index, _)) in content[next_line_start..].char_indices().enumerate() {
                    if i == new_col {
                        byte_offset = next_line_start + b_index;
                        break;
                    }
                }
                view.set_cursor(byte_offset);
            });
        });

        // Move the cursor up when Ctrl+w is pressed.
        siv.add_global_callback(Event::CtrlChar('w'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();

                // Find the start of the current line.
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                // Calculate the current column.
                let col = content[current_line_start..cur].chars().count();

                // If there is no previous line, do nothing.
                if current_line_start == 0 {
                    return;
                }

                // Determine the start of the previous line.
                let prev_line_start = content[..current_line_start - 1]
                    .rfind('\n')
                    .map(|pos| pos + 1)
                    .unwrap_or(0);
                // Get the number of characters in the previous line.
                let prev_line_length = content[prev_line_start..current_line_start - 1]
                    .chars()
                    .count();
                // The new column is the lesser of the current column and the previous line's length.
                let new_col = col.min(prev_line_length);

                // Convert the new column to a byte offset.
                let mut byte_offset = prev_line_start;
                for (i, (b_index, _)) in content[prev_line_start..].char_indices().enumerate() {
                    if i == new_col {
                        byte_offset = prev_line_start + b_index;
                        break;
                    }
                }
                view.set_cursor(byte_offset);
            });
        });

        // -------------------------------------------------
        // Toggle Selection with Ctrl+Space
        // -------------------------------------------------
        //
        // If no selection is active, this will select the character at the
        // current cursor position by inserting the markers "<|" and "|>".
        // If a selection is active, it will remove the markers and clear the selection.
        siv.add_global_callback(Event::CtrlChar(' '), move |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                // Store the original cursor position before modifying the content.
                let orig_cursor = view.cursor();
                let content = view.get_content();

                if self.selection.is_empty() {
                    // ---------- Selection (Highlight) ----------
                    // If no selection is active, select the character at the cursor.
                    if orig_cursor < content.len() {
                        if let Some(ch) = content[orig_cursor..].chars().next() {
                            let char_len = ch.len_utf8();
                            let end = orig_cursor + char_len;
                            // Update the selection state with the chosen character.
                            self.update_selection(content.to_string(), orig_cursor, end);
                            // Rebuild the content with markers inserted around the selected character.
                            let new_content = format!(
                                "{}<|{}|>{}",
                                &content[..orig_cursor],
                                self.selection,
                                &content[end..]
                            );
                            view.set_content(new_content);
                            // Adjust the cursor to be positioned after the opening marker.
                            // (Here, 2 is the length of "<|".)
                            view.set_cursor(orig_cursor + 2);
                        }
                    }
                } else {
                    // ---------- Unselection (Remove Highlight) ----------
                    // When a selection is active, remove the inserted markers.
                    let marker = format!("<|{}|>", self.selection);
                    let new_content = content.replace(&marker, &self.selection);
                    view.set_content(new_content);
                    // Clear the selection state.
                    self.selection.clear();
                    self.selection_start = orig_cursor;
                    self.selection_end = orig_cursor;
                    // Adjust the cursor back by the length of the removed marker.
                    // Use `saturating_sub` to avoid underflow when at very low indices.
                    view.set_cursor(orig_cursor.saturating_sub(2));
                }
            });
        });

        // -------------------------------------------------
        // Transformation Menu with Ctrl+u
        // -------------------------------------------------
        //
        // Opens a menu that lets you transform the text in the main text area.
        // Options include converting the text to uppercase, lowercase, or capitalized.
        siv.add_global_callback(Event::CtrlChar('u'), |s| {
            // Create a SelectView for the transformation choices.
            let mut sv: SelectView<Choice> = SelectView::new();
            sv.add_item("Uppercase", Choice::Upper);
            sv.add_item("Lowercase", Choice::Lower);
            sv.add_item("Capitalized", Choice::Cap);

            // When a choice is submitted, transform the text accordingly.
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
                // Remove the transformation menu layer.
                s.pop_layer();
            });

            // Add the transformation menu as a new layer.
            s.add_layer(sv);
        });

        // Run the Cursive event loop.
        siv.run();
    }
}

/// Capitalizes each word in the provided text.
///
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

/// Entry point of the program.
fn main() {
    let editor = Editor::new();
    editor.run();
}

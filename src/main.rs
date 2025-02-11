use cursive::views::{SelectView, TextArea};
use cursive::{event::Event, traits::*};

#[derive(Clone, Copy, Debug)]
enum Choice {
    Upper,
    Lower,
    Cap,
}

struct Editor {
    selection: String,
}

impl Editor {
    fn new() -> Self {
        Self {
            selection: String::new(),
        }
    }

    fn run(mut self) {
        let mut siv = cursive::default();

        let main_text_area = TextArea::new().with_name("main").full_screen();
        siv.add_layer(main_text_area);

        //
        // Basic Movement with WASD
        //

        // Right
        siv.add_global_callback(Event::CtrlChar('d'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                // Don't move right if already at the end.
                if cur >= content.len() {
                    return;
                }
                // Get the next character from the current position.
                if let Some(next_char) = content[cur..].chars().next() {
                    let new_cursor = cur + next_char.len_utf8();
                    view.set_cursor(new_cursor);
                }
            });
        });

        // Left
        siv.add_global_callback(Event::CtrlChar('a'), |s| {
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();
                // Don't move left if already at the beginning.
                if cur == 0 {
                    return;
                }
                // Use `char_indices()` to get the last character’s start before the current cursor.
                let new_cursor = content[..cur]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                view.set_cursor(new_cursor);
            });
        });

        // Down
        siv.add_global_callback(Event::CtrlChar('s'), |s| {
            // 'n' for next/down
            s.call_on_name("main", |view: &mut TextArea| {
                let content = view.get_content();
                let cur = view.cursor();

                // Find the start of the current line.
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);

                // Compute the column (number of characters from the current line's start).
                let col = content[current_line_start..cur].chars().count();

                // Find the end of the current line (the next newline character).
                let current_line_end = content[cur..]
                    .find('\n')
                    .map(|pos| cur + pos)
                    .unwrap_or(content.len());

                // Check if there is a next line. If not, we can't move down.
                if current_line_end >= content.len() {
                    return;
                }

                // The start of the next line is one byte after the newline.
                let next_line_start = current_line_end + 1;

                // Find the end of the next line.
                let next_line_end = content[next_line_start..]
                    .find('\n')
                    .map(|pos| next_line_start + pos)
                    .unwrap_or(content.len());

                // Determine the length (in characters) of the next line.
                let next_line_length = content[next_line_start..next_line_end].chars().count();

                // The desired column is the same as the current one unless the next line is shorter.
                let new_col = col.min(next_line_length);

                // Convert the new column (character count) into a byte offset in the next line.
                let mut byte_offset = next_line_start;
                for (i, (b_index, _)) in content[next_line_start..].char_indices().enumerate() {
                    if i == new_col {
                        byte_offset = next_line_start + b_index;
                        break;
                    }
                }

                // Finally, set the new cursor position.
                view.set_cursor(byte_offset);
            });
        });

        // Up
        siv.add_global_callback(Event::CtrlChar('w'), |s| {
            // 'p' for previous/up
            s.call_on_name("main", |view: &mut TextArea| {
                // Get the current content and cursor position.
                let content = view.get_content();
                let cur = view.cursor();

                // Find the beginning of the current line.
                let current_line_start = content[..cur].rfind('\n').map(|pos| pos + 1).unwrap_or(0);

                // Compute the column (number of characters from line start).
                let col = content[current_line_start..cur].chars().count();

                // Cannot go up if there is no previous line.
                if current_line_start == 0 {
                    return;
                }

                // Find the beginning of the previous line.
                // Look for the newline before the current line start.
                let prev_line_start = content[..current_line_start - 1]
                    .rfind('\n')
                    .map(|pos| pos + 1)
                    .unwrap_or(0);

                // Determine the previous line’s length in characters.
                let prev_line_length = content[prev_line_start..current_line_start - 1]
                    .chars()
                    .count();

                // The new column should be the same as the current one,
                // unless the previous line is shorter.
                let new_col = col.min(prev_line_length);

                // Now, convert the new_col (a character count) into a byte offset.
                let mut byte_offset = prev_line_start;
                for (i, (b_index, _)) in content[prev_line_start..].char_indices().enumerate() {
                    if i == new_col {
                        byte_offset = prev_line_start + b_index;
                        break;
                    }
                    // If we get to the end, byte_offset will be at the end of the line.
                }

                // Finally, set the new cursor position.
                view.set_cursor(byte_offset);
            });
        });

        siv.add_global_callback(Event::CtrlChar(' '), move |s| {
            // Handle space key press here
            s.call_on_name("main", |view: &mut TextArea| {
                if self.selection == "" {
                    self.selection = view.get_content().to_string();
                    let new_content = format!("<|{}|>", self.selection);
                    view.set_content(new_content);
                } else {
                    let unselect = format!("<|{}|>", self.selection);
                    let new_content = view.get_content().replace(&unselect, &self.selection);
                    view.set_content(new_content);
                    self.selection = "".to_string();
                }
            });
        });

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

fn main() {
    let editor = Editor::new();

    editor.run();
}

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

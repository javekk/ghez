use macroquad::{
    input::{KeyCode, clear_input_queue, get_char_pressed, is_key_down, is_key_pressed},
    miniquad::window::clipboard_get,
    time::get_time,
};

use crate::game::fen;

const REPEAT_DELAY: f64 = 0.4; // seconds before a held backspace starts repeating
const REPEAT_RATE: f64 = 0.03; // seconds between repeated deletions

pub enum Dialog {
    FenInput(FenInput),
    ConfirmNewGame(Option<String>), // FEN to start from, None for the initial position
}

pub enum DialogOutcome {
    Pending,
    Confirmed,
    Cancelled,
}

pub struct FenInput {
    pub text: String,
    pub error: Option<String>,
    backspace_down_at: Option<f64>,
    last_backspace_at: f64,
}

impl FenInput {
    pub fn new() -> Self {
        // Drop keystrokes typed before the dialog was opened
        clear_input_queue();
        Self {
            text: String::new(),
            error: None,
            backspace_down_at: None,
            last_backspace_at: 0.,
        }
    }

    /// Consumes this frame's keyboard input and edits the text accordingly.
    pub fn handle_keys(&mut self) {
        let shortcut = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);

        // The char queue is a stack: collect then reverse to keep typing order
        let mut chars = Vec::new();
        while let Some(c) = get_char_pressed() {
            chars.push(c);
        }
        if !shortcut {
            for c in chars.into_iter().rev() {
                if c.is_ascii_graphic() || c == ' ' {
                    self.edit(|text| text.push(c));
                }
            }
        }

        if shortcut
            && is_key_pressed(KeyCode::V)
            && let Some(pasted) = clipboard_get()
        {
            let pasted: String = pasted.lines().collect::<Vec<_>>().join(" ");
            self.edit(|text| text.push_str(pasted.trim()));
        }

        self.handle_backspace();
    }

    /// Validates the FEN, storing the error to show if it is invalid.
    pub fn submit(&mut self) -> Option<String> {
        let candidate = self.text.trim().to_string();
        if candidate.is_empty() {
            self.error = Some("Please enter a FEN".to_string());
            return None;
        }
        match fen::parse(&candidate) {
            Ok(_) => Some(candidate),
            Err(e) => {
                self.error = Some(e);
                None
            }
        }
    }

    fn handle_backspace(&mut self) {
        let now = get_time();
        if is_key_pressed(KeyCode::Backspace) {
            self.backspace_down_at = Some(now);
            self.last_backspace_at = now;
            self.edit(|text| {
                text.pop();
            });
        } else if !is_key_down(KeyCode::Backspace) {
            self.backspace_down_at = None;
        } else if let Some(down_at) = self.backspace_down_at
            && now - down_at > REPEAT_DELAY
            && now - self.last_backspace_at > REPEAT_RATE
        {
            self.last_backspace_at = now;
            self.edit(|text| {
                text.pop();
            });
        }
    }

    fn edit(&mut self, f: impl FnOnce(&mut String)) {
        f(&mut self.text);
        self.error = None;
    }
}

/// Enter confirms, Escape cancels.
pub fn keyboard_outcome() -> DialogOutcome {
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        DialogOutcome::Confirmed
    } else if is_key_pressed(KeyCode::Escape) {
        DialogOutcome::Cancelled
    } else {
        DialogOutcome::Pending
    }
}

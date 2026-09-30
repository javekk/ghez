use macroquad::{
    input::{
        KeyCode, MouseButton, clear_input_queue, get_char_pressed, is_key_down, is_key_pressed,
        is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released, mouse_position,
    },
    math::Vec2,
    miniquad::window::clipboard_get,
};

use crate::{
    game::{
        domain::{Piece, PieceType, Square},
        fen,
        game::Game,
    },
    render::theme,
};

#[derive(Clone)]

pub struct Drag {
    pub from: Square,
    pub piece: Piece,
    pub mouse_pos: (f32, f32), // Current cursor px
    pub legal_moves: Vec<Square>,
}

pub enum InputStatus {
    Chilling,
    Dragging(Drag),
    Releasing(Drag, Option<Square>),
    FiringNewGame(Option<String>), // New game from fen or normal game
    Promoting(Option<PieceType>),  // Promotion piece chosen, None if cancelled
}

pub enum Dialog {
    FenInput { text: String, error: Option<String> },
    ConfirmNewGame(Option<String>), // FEN to start from, None for the initial position
}

pub struct InputHandler {
    drag: Option<Drag>,
    dialog: Option<Dialog>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            drag: None,
            dialog: None,
        }
    }

    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    pub fn poll(&mut self, game: &Game) -> InputStatus {
        if game.pending_promotion.is_some() {
            self.drag = None;
            Self::on_promotion()
        } else if self.dialog.is_some() {
            self.on_dialog()
        } else if Self::is_in_shell() && self.drag.is_none() {
            self.on_shell(game)
        } else {
            self.on_board(game)
        }
    }

    fn mouse_world() -> Vec2 {
        theme::ui_camera().screen_to_world(mouse_position().into())
    }

    fn is_in_shell() -> bool {
        Self::mouse_world().x > theme::VIRTUAL_H
    }

    fn on_shell(&mut self, _game: &Game) -> InputStatus {
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse = Self::mouse_world();
            if theme::new_game_button().contains(mouse) {
                self.dialog = Some(Dialog::ConfirmNewGame(None));
            } else if theme::from_fen_button().contains(mouse) {
                clear_input_queue(); // drop keys typed before the dialog opened
                self.dialog = Some(Dialog::FenInput {
                    text: String::new(),
                    error: None,
                });
            }
        }
        InputStatus::Chilling
    }

    fn on_promotion() -> InputStatus {
        if is_key_pressed(KeyCode::Escape) {
            return InputStatus::Promoting(None);
        }
        for (key, kind) in [
            (KeyCode::Q, PieceType::Queen),
            (KeyCode::R, PieceType::Rook),
            (KeyCode::B, PieceType::Bishop),
            (KeyCode::N, PieceType::Knight),
        ] {
            if is_key_pressed(key) {
                return InputStatus::Promoting(Some(kind));
            }
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse = Self::mouse_world();
            if let Some((kind, _)) = theme::promotion_choices()
                .into_iter()
                .find(|(_, rect)| rect.contains(mouse))
            {
                return InputStatus::Promoting(Some(kind));
            }
            if theme::promotion_cancel_button().contains(mouse) {
                return InputStatus::Promoting(None);
            }
        }
        InputStatus::Chilling
    }

    fn on_dialog(&mut self) -> InputStatus {
        let click = is_mouse_button_pressed(MouseButton::Left);
        let mouse = Self::mouse_world();
        let confirm = is_key_pressed(KeyCode::Enter)
            || (click && theme::dialog_confirm_button().contains(mouse));
        let cancel = is_key_pressed(KeyCode::Escape)
            || (click && theme::dialog_cancel_button().contains(mouse));

        match self.dialog.take() {
            _ if cancel => {}
            Some(Dialog::ConfirmNewGame(fen)) if confirm => {
                return InputStatus::FiringNewGame(fen);
            }
            Some(Dialog::FenInput { mut text, error }) => {
                Self::edit_text(&mut text);
                self.dialog = Some(match fen::parse(text.trim()) {
                    Ok(_) if confirm => Dialog::ConfirmNewGame(Some(text.trim().to_string())),
                    Err(e) if confirm => Dialog::FenInput {
                        text,
                        error: Some(e),
                    },
                    _ => Dialog::FenInput { text, error },
                });
            }
            dialog => self.dialog = dialog,
        }
        InputStatus::Chilling
    }

    fn edit_text(text: &mut String) {
        let shortcut = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::LeftSuper);
        // The char queue is a stack: reverse to keep typing order
        let mut typed: Vec<char> = std::iter::from_fn(get_char_pressed).collect();
        typed.reverse();

        if shortcut {
            if is_key_pressed(KeyCode::V) {
                text.push_str(clipboard_get().unwrap_or_default().trim());
            }
        } else {
            text.extend(
                typed
                    .into_iter()
                    .filter(|c| c.is_ascii_graphic() || *c == ' '),
            );
        }
        if is_key_pressed(KeyCode::Backspace) {
            text.pop();
        }
    }

    fn on_board(&mut self, game: &Game) -> InputStatus {
        let pos: (f32, f32) = theme::ui_camera()
            .screen_to_world(mouse_position().into())
            .into();

        if is_mouse_button_pressed(MouseButton::Left) && self.drag.is_none() {
            let Some(square) = Self::pixel_to_square(pos) else {
                println!("No square selected");
                return InputStatus::Chilling;
            };
            println!("Selected square: {:?}", square);

            let Some(piece) = game.get_piece(square) else {
                println!("No piece selected");
                return InputStatus::Chilling;
            };
            println!("Selected piece: {:?}", piece);
            let drag = Drag {
                from: square,
                piece,
                mouse_pos: pos,
                legal_moves: game.get_legal_moves(piece, square),
            };
            self.drag = Some(drag.clone());
            return InputStatus::Dragging(drag.clone());
        }

        if is_mouse_button_released(MouseButton::Left) && self.drag.is_some() {
            let Some(drag) = self.drag.take() else {
                println!("This seems an error state");
                return InputStatus::Chilling;
            };

            if let Some(square) = Self::pixel_to_square(pos) {
                println!("Selected squar on release: {:?}", square);
                return InputStatus::Releasing(drag.clone(), Some(square));
            } else {
                println!("No square selected on release");
                return InputStatus::Releasing(drag.clone(), None);
            };
        }

        if is_mouse_button_down(MouseButton::Left) && self.drag.is_some() {
            if let Some(drag) = self.drag.as_ref() {
                let new_drag = Drag {
                    from: drag.from,
                    piece: drag.piece,
                    mouse_pos: mouse_position(),
                    legal_moves: drag.legal_moves.clone(),
                };
                return InputStatus::Dragging(new_drag);
            }
        }

        InputStatus::Chilling
    }

    fn pixel_to_square(mouse_position: (f32, f32)) -> Option<Square> {
        let file = mouse_position.0 / theme::SQUARE_SIZE as f32;
        let rank = mouse_position.1 / theme::SQUARE_SIZE as f32;

        let ufile = file as i8;

        if ufile > 7 {
            return None;
        }

        let urank = 7 - (rank as i8);

        if urank > 7 {
            return None;
        }

        let idx = urank * 8 + ufile;

        Square::from_index(idx)
    }
}

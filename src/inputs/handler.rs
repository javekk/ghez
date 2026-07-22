use macroquad::{
    input::{
        MouseButton, is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released,
        mouse_position,
    },
    math::Vec2,
};

use crate::{
    game::{
        domain::{Piece, Square},
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
}

pub struct InputHandler {
    drag: Option<Drag>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self { drag: None }
    }

    pub fn poll(&mut self, game: &Game) -> InputStatus {
        if Self::is_in_shell() && self.drag.is_none() {
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
        if is_mouse_button_pressed(MouseButton::Left)
            && theme::new_game_button().contains(Self::mouse_world())
        {
            return InputStatus::FiringNewGame(None);
        }
        InputStatus::Chilling
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

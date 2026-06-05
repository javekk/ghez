use macroquad::input::{
    MouseButton, is_mouse_button_pressed, is_mouse_button_released, mouse_position,
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
    pub mouse: (f32, f32), // Current cursor px
}

pub enum InputStatus {
    Chilling,
    Dragging(Drag),
    Releasing(Drag, Square),
}

pub struct InputHandler {
    drag: Option<Drag>,
    mouse_position: (f32, f32),
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            drag: None,
            mouse_position: (0., 0.),
        }
    }

    pub fn poll(&mut self, game: &Game) -> InputStatus {
        self.mouse_position = mouse_position();

        if is_mouse_button_pressed(MouseButton::Left) && self.drag.is_none() {
            let Ok(square) = Self::pixel_to_square(self.mouse_position) else {
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
                mouse: self.mouse_position,
            };
            self.drag = Some(drag.clone());
            return InputStatus::Dragging(drag.clone());
        }

        if is_mouse_button_released(MouseButton::Left) && self.drag.is_some() {
            let Some(drag) = self.drag.take() else {
                println!("This seems an error state");
                return InputStatus::Chilling;
            };

            let Ok(square) = Self::pixel_to_square(self.mouse_position) else {
                println!("No square selected on release");
                return InputStatus::Chilling;
            };
            println!("Selected squar on release: {:?}", square);

            if square != drag.from {
                println!(
                    "Move piece {:?}, from: {:?}, to: {:?}",
                    drag.piece, drag.from, square
                );
            }
            return InputStatus::Releasing(drag.clone(), square);
        }
        InputStatus::Chilling
    }

    fn pixel_to_square(mouse_position: (f32, f32)) -> Result<Square, ()> {
        // TODO check that we are inside the board

        let file = mouse_position.0 / theme::SQUARE_SIZE as f32;
        let rank = mouse_position.1 / theme::SQUARE_SIZE as f32;

        let ufile = file as i8;
        let urank = 7 - (rank as i8);
        let idx = urank * 8 + ufile;

        Square::try_from(idx)
    }
}

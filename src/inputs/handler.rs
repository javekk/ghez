use macroquad::input::{MouseButton, is_mouse_button_pressed, mouse_position};

use crate::{
    game::{
        domain::{Piece, Square},
        game_state::GameState,
    },
    render::theme,
};

pub struct Drag {
    pub from: Square,
    pub piece: Piece,
    pub mouse: (f32, f32), // Current cursor px
}

pub enum Action {
    None,
    BeginDrag { from: usize },
    DropOnSquare { from: usize, to: usize },
    DropOffBoard { from: usize },
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

    pub fn drag(&self) -> Option<(&Drag, (f32, f32))> {
        self.drag.as_ref().map(|d| (d, self.mouse_position))
    }

    pub fn poll(&mut self, state: &GameState) -> Action {
        self.mouse_position = mouse_position();

        if is_mouse_button_pressed(MouseButton::Left) && self.drag.is_none() {
            println!("x: {}, y: {}", self.mouse_position.0, self.mouse_position.1);
            InputHandler::pixel_to_square(self.mouse_position);
        }
        Action::None
    }

    fn pixel_to_square(mouse_position: (f32, f32)) {
        let file = mouse_position.0 / theme::SQUARE_SIZE as f32;
        let rank = mouse_position.1 / theme::SQUARE_SIZE as f32;

        let ufile = file as i8;
        let urank = 7 - (rank as i8);
        let idx = urank * 8 + ufile;

        let square: Square = Square::try_from(idx).unwrap();
        println!("rank: {}, col: {}", urank, ufile);
        println!("Square: {:?}", square)
    }
}

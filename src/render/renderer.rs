use crate::game::domain::{Board, Piece, Side, Square};
use crate::game::game_state::GameState;
use crate::inputs;
use crate::inputs::handler::InputStatus;
use crate::render::theme;

use macroquad::prelude::*;
use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
enum TextureColor {
    Black,
    White,
    Grey,
    LightGrey,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    LightBlue,
    Violet,
}

pub struct Renderer {
    texture: Texture2D,
    texture_set_map: HashMap<TextureColor, Rect>,
    texture_cols: f32,
    texture_color_ligth: TextureColor,
    texture_color_dark: TextureColor,
}

impl Renderer {
    pub async fn new() -> Self {
        // Load main window
        request_new_screen_size(
            ((theme::SQUARE_SIZE * 8) + theme::BORDER / 2) as f32,
            ((theme::SQUARE_SIZE * 8) + theme::BORDER / 2) as f32,
        );

        // Load piece sets
        let texture = load_texture("assets/pieces/chess_sprites.png")
            .await
            .unwrap();

        let texture_rows: f32 = 12.0;
        let texture_cols: f32 = 6.0;

        let mut texture_set_map = HashMap::new();

        let mut i: f32 = 0.;
        for color in [
            TextureColor::Black,
            TextureColor::White,
            TextureColor::Grey,
            TextureColor::LightGrey,
            TextureColor::Pink,
            TextureColor::Red,
            TextureColor::Orange,
            TextureColor::Yellow,
            TextureColor::Green,
            TextureColor::Blue,
            TextureColor::LightBlue,
            TextureColor::Violet,
        ] {
            let piece_set_w = texture.width();
            let piece_set_h = texture.height() / texture_rows;
            let rect = Rect::new(0., i * piece_set_h, piece_set_w, piece_set_h);
            texture_set_map.insert(color, rect);
            i += 1.;
        }

        let texture_color_ligth = TextureColor::LightGrey;
        let texture_color_dark = TextureColor::Blue;

        Self {
            texture_set_map,
            texture,
            texture_cols,
            texture_color_ligth,
            texture_color_dark,
        }
    }

    fn square_to_pixel(i: usize) -> (f32, f32) {
        let square = theme::SQUARE_SIZE as f32;
        let border = (theme::BORDER_SIZE / 2) as f32;
        let x = (i % 8) as f32 * square + border;
        let y = ((63 - i) / 8) as f32 * square + border;
        (x, y)
    }

    fn get_piece_texture(&self, piece: Piece) -> DrawTextureParams {
        let texture_color = match piece.side {
            Side::White => self.texture_color_ligth,
            Side::Black => self.texture_color_dark,
        };

        let texture_set = self.texture_set_map.get(&texture_color).unwrap();

        let piece_w = self.texture.width() / self.texture_cols;
        let piece_h = texture_set.h;
        let piece_idx = piece.kind as usize;
        let piece_texture_rect = Rect::new(
            texture_set.x + piece_w * piece_idx as f32,
            texture_set.y,
            piece_w,
            piece_h,
        );

        DrawTextureParams {
            source: Some(piece_texture_rect),
            dest_size: Some(vec2(theme::SQUARE_SIZE as f32, theme::SQUARE_SIZE as f32)),
            ..Default::default()
        }
    }

    fn draw_piece_to_usquare(&self, piece: Piece, usquare: usize) {
        let (piece_texture_x, piece_texture_y) = Renderer::square_to_pixel(usquare);
        draw_texture_ex(
            &self.texture,
            piece_texture_x,
            piece_texture_y,
            WHITE,
            self.get_piece_texture(piece),
        );
    }

    fn draw_dragged_piece(&self, piece: Piece, pos: (f32, f32)) {
        let square = theme::SQUARE_SIZE as f32;

        draw_texture_ex(
            &self.texture,
            pos.0 - square / 2.,
            pos.1 - square / 2.,
            WHITE,
            self.get_piece_texture(piece),
        );
    }

    pub async fn run(&self, game_state: &GameState, input_status: &InputStatus) {
        clear_background(theme::BORDER_COLOR);

        // Draw cells
        let mut is_light = true; // start from a8
        for row in 0..8 {
            for col in 0..8 {
                let color = if is_light { theme::LIGHT } else { theme::DARK };
                let offset_x = (col * theme::SQUARE_SIZE) as f32;
                let offset_y = (row * theme::SQUARE_SIZE) as f32;
                draw_rectangle(
                    offset_x,
                    offset_y,
                    theme::SQUARE_SIZE as f32,
                    theme::SQUARE_SIZE as f32,
                    color,
                );

                is_light = !is_light;
            }
            is_light = !is_light;
        }

        // Draw boarders
        for row in 0..8 {
            let x1 = 0.;
            let y1 = (row * theme::SQUARE_SIZE) as f32;
            let x2 = 8. * theme::SQUARE_SIZE as f32;
            let y2 = (row * theme::SQUARE_SIZE) as f32;
            draw_line(x1, y1, x2, y2, theme::BORDER as f32, theme::BORDER_COLOR);
        }

        for col in 0..8 {
            let x1 = (col * theme::SQUARE_SIZE) as f32;
            let y1 = 0.;
            let x2 = (col * theme::SQUARE_SIZE) as f32;
            let y2 = 8. * theme::SQUARE_SIZE as f32;
            draw_line(x1, y1, x2, y2, theme::BORDER as f32, theme::BORDER_COLOR);
        }

        // Draw pieces

        for (i, piece_option) in game_state.board.iter().enumerate() {
            if let Some(piece) = piece_option {
                if let inputs::handler::InputStatus::Dragging(drag) = &input_status {
                    if i == drag.from as usize {
                        // skip drawing this piece at its square — it's being dragged
                        continue;
                    }
                }
                self.draw_piece_to_usquare(*piece, i);
            }
        }

        // Drag
        if let inputs::handler::InputStatus::Dragging(drag) = &input_status {
            self.draw_dragged_piece(drag.piece, drag.mouse_pos);
        }

        next_frame().await
    }
}

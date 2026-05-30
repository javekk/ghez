use macroquad::prelude::*;
use std::collections::HashMap;

use crate::render::theme;

mod render {
    pub mod theme;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    White,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

type Board = [Option<Piece>; 64]; // a1,a2,...,h7,h8

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Piece {
    side: Side,
    kind: PieceType,
}

fn square_to_pixel(i: usize) -> (f32, f32) {
    let square = theme::SQUARE_SIZE as f32;
    let border = (theme::BORDER_SIZE / 2) as f32;
    let x = (i % 8) as f32 * square + border;
    let y = ((63 - i) / 8) as f32 * square + border;
    (x, y)
}

#[macroquad::main("Ghez")]
async fn main() {
    // init board
    let mut board: Board = [None; 64];

    // Load main window
    request_new_screen_size(
        ((theme::SQUARE_SIZE * 8) + theme::BORDER / 2) as f32,
        ((theme::SQUARE_SIZE * 8) + theme::BORDER / 2) as f32,
    );

    // Load piece sets
    let texture = load_texture("assets/pieces/chess_sprites.png")
        .await
        .unwrap();

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

    const TEXTURE_ROWS: f32 = 12.0;
    const TEXTURE_COLS: f32 = 6.0;
    let mut texture_set_map: HashMap<TextureColor, Rect> = HashMap::new();

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
        let piece_set_h = texture.height() / TEXTURE_ROWS;
        let rect = Rect::new(0., i * piece_set_h, piece_set_w, piece_set_h);
        texture_set_map.insert(color, rect);
        i += 1.;
    }

    loop {
        clear_background(theme::BORDER_COLOR);

        // ++++++++++++++ Draw board ++++++++++++++
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

        // ++++++++++++++ Draw Pieces ++++++++++++++

        // parse fen
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let fen_parts: Vec<&str> = fen.split(' ').collect();
        let fen_board = fen_parts[0];

        let mut rank_index = 7;
        let mut file_index = 0;
        for c in fen_board.chars() {
            if c == '/' {
                file_index = 0;
                rank_index -= 1;
            } else if c.is_digit(10) {
                file_index += c.to_digit(10).unwrap() as usize;
            } else {
                let side = if c.is_uppercase() {
                    Side::White
                } else {
                    Side::Black
                };
                let piece_type = match c.to_ascii_lowercase() {
                    'k' => PieceType::King,
                    'q' => PieceType::Queen,
                    'r' => PieceType::Rook,
                    'b' => PieceType::Bishop,
                    'n' => PieceType::Knight,
                    'p' => PieceType::Pawn,
                    _ => panic!("Invalid piece type in FEN"),
                };
                let piece = Piece {
                    side,
                    kind: piece_type,
                };
                board[file_index + (rank_index * 8)] = Some(piece);
                file_index += 1;
            }
        }

        // Draw pieces

        let texture_color_ligth = TextureColor::LightGrey;
        let texture_color_dark = TextureColor::Blue;
        for (i, piece_option) in board.iter().enumerate() {
            if let Some(piece) = piece_option {
                let texture_color = match piece.side {
                    Side::White => texture_color_ligth,
                    Side::Black => texture_color_dark,
                };

                let texture_set = texture_set_map.get(&texture_color).unwrap();

                let piece_w = texture.width() / TEXTURE_COLS;
                let piece_h = texture_set.h;
                let piece_idx = piece.kind as usize;
                let piece_texture_rect = Rect::new(
                    texture_set.x + piece_w * piece_idx as f32,
                    texture_set.y,
                    piece_w,
                    piece_h,
                );

                let (piece_texture_x, piece_texture_y) = square_to_pixel(i);
                draw_texture_ex(
                    &texture,
                    piece_texture_x,
                    piece_texture_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(piece_texture_rect),
                        dest_size: Some(vec2(theme::SQUARE_SIZE as f32, theme::SQUARE_SIZE as f32)),
                        ..Default::default()
                    },
                );
            }
        }

        next_frame().await
    }
}

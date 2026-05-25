use macroquad::color::Color;

pub const PIECE_PIXELS: u32 = 16;
pub const SCALE: u32 = 6;

pub const PIECE_SIZE: u32 = PIECE_PIXELS * SCALE;
pub const BORDER: u32 = SCALE;
pub const SQUARE_SIZE: u32 = PIECE_SIZE + BORDER;
pub const CELL: u32 = PIECE_SIZE;

pub const LIGHT: Color = Color::from_rgba(213, 191, 180, 255);
pub const DARK: Color = Color::from_rgba(225, 173, 1, 255);
pub const BORDER_COLOR: Color = Color::from_rgba(30, 20, 10, 255);
pub const HIGHLIGHT: Color = Color::from_rgba(100, 200, 100, 160);
pub const LEGAL_DOT: Color = Color::from_rgba(0, 0, 0, 60);

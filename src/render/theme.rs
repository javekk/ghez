use macroquad::{
    camera::Camera2D,
    color::Color,
    math::Rect,
    window::{screen_height, screen_width},
};

pub const PIECE_PIXELS: u32 = 16;
pub const SCALE: u32 = 6;

pub const PIECE_SIZE: u32 = PIECE_PIXELS * SCALE;
pub const BORDER_SIZE: u32 = 1;
pub const BORDER: u32 = SCALE * BORDER_SIZE;
pub const SQUARE_SIZE: u32 = PIECE_SIZE + BORDER;

pub const LIGHT: Color = Color::from_rgba(190, 208, 193, 255);
pub const DARK: Color = Color::from_rgba(116, 121, 87, 255);
pub const BORDER_COLOR: Color = Color::from_rgba(121, 57, 59, 255);
pub const LEGAL_DOT: Color = Color::from_rgba(0, 0, 0, 60);

pub const VIRTUAL_H: f32 = (SQUARE_SIZE * 8) as f32;

pub fn ui_camera() -> Camera2D {
    // keep virtual height fixed; let virtual width follow the window's aspect
    // so squares stay square (extra space shows up on the right)
    let w = VIRTUAL_H * screen_width() / screen_height();
    Camera2D::from_display_rect(Rect::new(0., VIRTUAL_H, w, -VIRTUAL_H))
}

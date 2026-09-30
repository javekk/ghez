use macroquad::{
    camera::Camera2D,
    color::{Color, GRAY, LIGHTGRAY},
    math::Rect,
    window::{screen_height, screen_width},
};

use crate::game::domain::PieceType;

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

// SHELL

pub const FONT_SIZE: u32 = 24;
pub const BUTTON_COLOR: Color = GRAY;
pub const BUTTON_COLOR_HIGHLIGHT: Color = LIGHTGRAY;
pub const BUTTON_W: f32 = 130.;
pub const BUTTON_H: f32 = 32.;
pub const SHELL_PAD: f32 = 16.;

pub const SHELL_W: f32 = BUTTON_W + SHELL_PAD * 2.;

pub fn new_game_button() -> Rect {
    Rect::new(VIRTUAL_H + SHELL_PAD, SHELL_PAD, BUTTON_W, BUTTON_H)
}

pub fn from_fen_button() -> Rect {
    Rect::new(
        VIRTUAL_H + SHELL_PAD,
        SHELL_PAD * 2. + BUTTON_H,
        BUTTON_W,
        BUTTON_H,
    )
}

// DIALOG

pub const OVERLAY_COLOR: Color = Color::from_rgba(0, 0, 0, 150);
pub const DIALOG_COLOR: Color = Color::from_rgba(48, 46, 43, 255);
pub const ERROR_COLOR: Color = Color::from_rgba(230, 90, 80, 255);
pub const SMALL_FONT_SIZE: u32 = 16;

pub fn dialog_rect() -> Rect {
    Rect::new((VIRTUAL_W - 760.) / 2., (VIRTUAL_H - 200.) / 2., 760., 200.)
}

pub fn dialog_confirm_button() -> Rect {
    let d = dialog_rect();
    Rect::new(
        d.right() - SHELL_PAD - BUTTON_W,
        d.bottom() - SHELL_PAD - BUTTON_H,
        BUTTON_W,
        BUTTON_H,
    )
}

pub fn dialog_cancel_button() -> Rect {
    let confirm = dialog_confirm_button();
    Rect::new(
        confirm.x - SHELL_PAD - BUTTON_W,
        confirm.y,
        BUTTON_W,
        BUTTON_H,
    )
}

/// Cancel button of the promotion dialog, on the title row so it never
/// overlaps the piece choices.
pub fn promotion_cancel_button() -> Rect {
    let d = dialog_rect();
    Rect::new(
        d.right() - SHELL_PAD - BUTTON_W,
        d.y + SHELL_PAD,
        BUTTON_W,
        BUTTON_H,
    )
}

/// Clickable piece choices of the promotion dialog, left to right.
pub fn promotion_choices() -> [(PieceType, Rect); 4] {
    let d = dialog_rect();
    let size = SQUARE_SIZE as f32;
    let gap = SHELL_PAD;
    let total = size * 4. + gap * 3.;
    let x = d.x + (d.w - total) / 2.;
    let y = d.y + 60.;
    [
        PieceType::Queen,
        PieceType::Rook,
        PieceType::Bishop,
        PieceType::Knight,
    ]
    .map(|kind| (kind, Rect::new(0., y, size, size)))
    .into_iter()
    .enumerate()
    .map(|(i, (kind, r))| {
        (
            kind,
            Rect::new(x + i as f32 * (size + gap), r.y, size, size),
        )
    })
    .collect::<Vec<_>>()
    .try_into()
    .unwrap()
}

// Window

pub const VIRTUAL_H: f32 = (SQUARE_SIZE * 8) as f32;
pub const VIRTUAL_W: f32 = VIRTUAL_H + SHELL_W;

pub fn ui_camera() -> Camera2D {
    let aspect = screen_width() / screen_height();
    let (w, h) = if aspect > VIRTUAL_W / VIRTUAL_H {
        (VIRTUAL_H * aspect, VIRTUAL_H) // wide window: extra space on the right
    } else {
        (VIRTUAL_W, VIRTUAL_W / aspect) // narrow/tall: extra space at the bottom
    };
    Camera2D::from_display_rect(Rect::new(0., h, w, -h))
}

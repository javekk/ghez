use crate::game::domain::{Piece, Side, Square};
use crate::game::game::Game;
use crate::game::game_state::{DrawReason, GameState, GameStatus};
use crate::inputs;
use crate::inputs::dialog::Dialog;
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
        request_new_screen_size(theme::VIRTUAL_W, theme::VIRTUAL_H);

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

    fn draw_squares(&self) {
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
    }

    fn draw_borders(&self) {
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
    }

    fn draw_pieces(&self, game_state: &GameState, input_status: &InputStatus) {
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

        let real_pos: (f32, f32) = theme::ui_camera().screen_to_world(pos.into()).into();

        draw_texture_ex(
            &self.texture,
            real_pos.0 - square / 2.,
            real_pos.1 - square / 2.,
            WHITE,
            self.get_piece_texture(piece),
        );
    }

    fn draw_legal_moves_dots(&self, squares: &Vec<Square>) {
        let radius = theme::SQUARE_SIZE as f32 * 0.15;
        for square in squares {
            let (x, y) = Self::square_to_pixel(*square as usize);
            let cx = x + theme::SQUARE_SIZE as f32 / 2.0;
            let cy = y + theme::SQUARE_SIZE as f32 / 2.0;
            draw_circle(cx, cy, radius, theme::LEGAL_DOT);
        }
    }

    fn draw_board(&self, game: &Game, input_status: &InputStatus) {
        self.draw_squares();
        self.draw_borders();
        self.draw_pieces(&game.game_state, input_status);

        if let InputStatus::Dragging(drag) = input_status {
            self.draw_legal_moves_dots(&drag.legal_moves);
        }
    }

    fn draw_shell(&self, _game: &Game, input_status: &InputStatus, dialog: Option<&Dialog>) {
        let interactive = dialog.is_none() && !matches!(input_status, InputStatus::Dragging(_));
        Self::draw_button("New Game", theme::new_game_button(), interactive);
        Self::draw_button("From FEN", theme::from_fen_button(), interactive);
    }

    pub async fn run(&self, game: &Game, input_status: &InputStatus, dialog: Option<&Dialog>) {
        set_camera(&theme::ui_camera());

        clear_background(theme::BORDER_COLOR);

        self.draw_board(game, input_status);
        self.draw_shell(game, input_status, dialog);
        if let Some(dialog) = dialog {
            Self::draw_dialog(dialog);
        }

        match game.parse_game_status() {
            GameStatus::Chilling => {}
            GameStatus::Battling => { /* Games is going on */ }
            GameStatus::Draw(draw_reason) => {
                println!("It's a draw");
                match draw_reason {
                    DrawReason::Stalemate => {
                        println!("Stalemate")
                    }
                    DrawReason::FiftyMoveRule => {
                        println!("FiftyMoveRule")
                    }
                    DrawReason::ThreefoldRepetition => {
                        println!("ThreefoldRepetition")
                    }
                    DrawReason::InsufficientMaterial => {
                        println!("InsufficientMaterial")
                    }
                    DrawReason::Agreement => todo!(),
                }
            }
            GameStatus::Mated(side) => {
                let winner = if side == Side::White {
                    "Black"
                } else {
                    "White"
                };
                println!("{} has won the game!", winner);
            }
            GameStatus::LostOnTime(_side) => todo!(),
            GameStatus::RunAway(_side) => todo!(),
        }

        next_frame().await
    }

    fn draw_button(label: &str, rect: Rect, interactive: bool) {
        let mouse = theme::ui_camera().screen_to_world(mouse_position().into());
        let hover = rect.contains(mouse);
        let background = if hover && interactive {
            theme::BUTTON_COLOR_HIGHLIGHT
        } else {
            theme::BUTTON_COLOR
        };

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, background);
        let dims = measure_text(label, None, theme::FONT_SIZE as u16, 1.0);
        draw_text(
            label,
            rect.x + (rect.w - dims.width) / 2.,
            rect.y + (rect.h + dims.offset_y) / 2.,
            theme::FONT_SIZE as f32,
            WHITE,
        );
    }

    fn draw_dialog(dialog: &Dialog) {
        // Dim everything behind the dialog, including any extra window space
        draw_rectangle(
            0.,
            0.,
            theme::VIRTUAL_W * 4.,
            theme::VIRTUAL_H * 4.,
            theme::OVERLAY_COLOR,
        );

        let panel = theme::dialog_rect();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, theme::DIALOG_COLOR);
        draw_rectangle_lines(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            theme::BORDER as f32,
            theme::DIALOG_BORDER_COLOR,
        );

        let text_x = panel.x + theme::DIALOG_PAD;
        let title_y = panel.y + theme::DIALOG_PAD + theme::FONT_SIZE as f32;

        match dialog {
            Dialog::ConfirmNewGame(fen) => {
                draw_text(
                    "Start a new game?",
                    text_x,
                    title_y,
                    theme::FONT_SIZE as f32,
                    WHITE,
                );
                let from = match fen {
                    Some(_) => "The new game will start from the given FEN.",
                    None => "The new game will start from the initial position.",
                };
                for (i, line) in ["The current game will be lost.", from].iter().enumerate() {
                    draw_text(
                        line,
                        text_x,
                        title_y + 40. + i as f32 * 28.,
                        theme::SMALL_FONT_SIZE as f32,
                        theme::HINT_COLOR,
                    );
                }
                Self::draw_button("Cancel", theme::dialog_cancel_button(), true);
                Self::draw_button("New Game", theme::dialog_confirm_button(), true);
            }
            Dialog::FenInput(input) => {
                draw_text(
                    "New game from FEN",
                    text_x,
                    title_y,
                    theme::FONT_SIZE as f32,
                    WHITE,
                );
                Self::draw_text_box(&input.text);

                let text_box = theme::dialog_text_box();
                let (message, color) = match &input.error {
                    Some(error) => (error.as_str(), theme::ERROR_COLOR),
                    None => (
                        "Enter: confirm   Esc: cancel   Ctrl+V: paste",
                        theme::HINT_COLOR,
                    ),
                };
                draw_text(
                    message,
                    text_box.x,
                    text_box.bottom() + 28.,
                    theme::SMALL_FONT_SIZE as f32,
                    color,
                );
                Self::draw_button("Cancel", theme::dialog_cancel_button(), true);
                Self::draw_button("OK", theme::dialog_confirm_button(), true);
            }
        }
    }

    fn draw_text_box(text: &str) {
        let rect = theme::dialog_text_box();
        let font_size = theme::SMALL_FONT_SIZE as f32;
        let pad = 8.;
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, theme::TEXT_BOX_COLOR);

        // Long FENs don't fit: keep the tail visible, where the caret is
        let max_w = rect.w - pad * 2. - 10.;
        let mut visible = text;
        while measure_text(visible, None, font_size as u16, 1.0).width > max_w {
            let mut chars = visible.chars();
            chars.next();
            visible = chars.as_str();
        }

        let baseline =
            rect.y + (rect.h + measure_text("Ag", None, font_size as u16, 1.0).offset_y) / 2.;
        draw_text(visible, rect.x + pad, baseline, font_size, WHITE);

        // Blinking caret
        if get_time() % 1.0 < 0.5 {
            let caret_x =
                rect.x + pad + measure_text(visible, None, font_size as u16, 1.0).width + 2.;
            draw_line(caret_x, rect.y + 8., caret_x, rect.bottom() - 8., 2., WHITE);
        }
    }
}

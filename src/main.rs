use macroquad::prelude::*;

use crate::render::theme;

mod render {
    pub mod theme;
}

#[macroquad::main("Ghez")]
async fn main() {
    loop {
        // Draw board
        request_new_screen_size(
            (theme::SQUARE_SIZE * 8) as f32,
            (theme::SQUARE_SIZE * 8) as f32,
        );
        clear_background(theme::BORDER_COLOR);

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

        next_frame().await
    }
}

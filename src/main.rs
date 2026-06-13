mod render {
    pub mod renderer;
    pub mod theme;
}

mod game {
    pub mod domain;
    pub mod game;
    pub mod game_state;
}

mod inputs {
    pub mod handler;
}

use crate::game::game::Game;
use crate::inputs::handler::InputHandler;
use crate::render::renderer::Renderer;

#[macroquad::main("Ghez")]
async fn main() {
    let fen = "8/1K6/1N6/4q3/4R3/4r3/8/Nk6 w - - 0 1";

    let mut game: Game = Game::new_game_from_fen(fen);
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    loop {
        let input_status = input_handler.poll(&game);
        game.parse_input(&input_status);
        renderer.run(&game.game_state, &input_status).await;
    }
}

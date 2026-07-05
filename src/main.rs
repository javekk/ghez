mod render {
    pub mod renderer;
    pub mod theme;
}

mod game {
    pub mod domain;
    pub mod fen;
    pub mod game;
    pub mod game_state;
    pub mod movegen;
}

mod inputs {
    pub mod handler;
}

use crate::game::game::Game;
use crate::inputs::handler::InputHandler;
use crate::render::renderer::Renderer;

#[macroquad::main("Ghez")]
async fn main() {
    let fen = "8/8/8/8/8/5k2/4p3/4K3 b - - 0 1";

    let mut game: Game = Game::new_game_from_fen(fen);
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    loop {
        let input_status = input_handler.poll(&game);
        game.parse_input(&input_status);
        renderer.run(&game, &input_status).await;
    }
}

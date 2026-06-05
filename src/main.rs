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
    let fen = "rnbqkbnr/pppp1ppp/4p3/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut game: Game = Game::new_game_from_fen(fen);
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    loop {
        match input_handler.poll(&game) {
            inputs::handler::InputStatus::Chilling => { /* Just chilling */ }
            inputs::handler::InputStatus::Dragging(drag) => {
                // TODO stuff here
                println!("DRAG DUDE")
            }
            inputs::handler::InputStatus::Releasing(drag, to) => {
                // TODO also here
                game.move_piece(drag.from, to);
            }
        };
        renderer.run(&game.game_state).await;
    }
}

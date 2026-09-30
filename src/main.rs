mod render {
    pub mod renderer;
    pub mod theme;
}

mod game {
    pub mod domain;
    pub mod draw_checker;
    pub mod fen;
    pub mod game;
    pub mod game_state;
    pub mod movegen;
}

mod inputs {
    pub mod handler;
    pub mod terminal;
}

use crate::game::game::Game;
use crate::inputs::handler::InputHandler;
use crate::inputs::terminal::{self, Command, Terminal};
use crate::render::renderer::Renderer;

#[macroquad::main("Ghez")]
async fn main() {
    let fen = "8/8/8/8/8/5k2/4p3/4K3 b - - 0 1";

    let mut game = Game::new_game_from_fen(&fen).unwrap_or_else(|e| {
        eprintln!("Invalid FEN: {e}"); // TODO set a UI status/toast string
        Game::new_game_from_initial_position()
    });
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    let terminal = Terminal::new();
    println!("Terminal commands: ng | ng <fen> | mv e2e4 | mv e7e8q (q/r/b/n) | exit (or Ctrl-C)");

    loop {
        for line in terminal.poll() {
            if terminal::parse_command(&line) == Ok(Command::Exit) {
                return;
            }
            game.run_command(&line);
        }
        let user_inputs = input_handler.poll(&game);
        game.parse_input(&user_inputs);
        renderer
            .run(&game, &user_inputs, input_handler.dialog())
            .await;
    }
}

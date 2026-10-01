mod render {
    pub mod renderer;
    pub mod theme;
}

mod engine {
    pub mod engine_match;
    pub mod uci;
}

mod game {
    pub mod domain;
    pub mod draw_checker;
    pub mod fen;
    pub mod game;
    pub mod game_state;
    pub mod movegen;
    pub mod notation;
}

mod inputs {
    pub mod handler;
    pub mod terminal;
}

use crate::engine::engine_match::{DEFAULT_MOVETIME_MS, EngineMatch};
use crate::game::domain::Side;
use crate::game::game::Game;
use crate::inputs::handler::{InputHandler, InputStatus};
use crate::inputs::terminal::{self, Command, Terminal};
use crate::render::renderer::Renderer;

/// Command-line options: `--engine <path> [--color white|black] [--movetime <ms>]`.
struct Options {
    engine: Option<String>,
    human: Side,
    movetime_ms: u64,
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        engine: None,
        human: Side::White,
        movetime_ms: DEFAULT_MOVETIME_MS,
    };
    let mut args = args;
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--engine" => options.engine = Some(value()?),
            "--color" => {
                options.human = match value()?.to_ascii_lowercase().as_str() {
                    "white" => Side::White,
                    "black" => Side::Black,
                    other => return Err(format!("invalid --color '{other}', use white or black")),
                }
            }
            "--movetime" => {
                options.movetime_ms = value()?
                    .parse()
                    .map_err(|_| "--movetime expects milliseconds".to_string())?
            }
            _ => return Err(format!("unknown option '{flag}'")),
        }
    }
    Ok(options)
}

fn start_engine(
    slot: &mut Option<EngineMatch>,
    path: &str,
    human: Side,
    movetime_ms: u64,
    game: &Game,
) {
    *slot = None; // stop the previous engine first
    match EngineMatch::start(path, human, movetime_ms, game) {
        Ok(engine) => {
            *slot = Some(engine);
            println!("Playing against {path} as {human:?} ({movetime_ms} ms per move)");
        }
        Err(e) => eprintln!("{e}"),
    }
}

#[macroquad::main("Ghez")]
async fn main() {
    let options = parse_options(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}\nusage: ghez [--engine <path>] [--color white|black] [--movetime <ms>]");
        std::process::exit(2);
    });

    let fen = "8/8/8/8/8/5k2/4p3/4K3 b - - 0 1";

    let mut game = if options.engine.is_some() {
        Game::new_game_from_initial_position()
    } else {
        Game::new_game_from_fen(&fen).unwrap_or_else(|e| {
            eprintln!("Invalid FEN: {e}"); // TODO set a UI status/toast string
            Game::new_game_from_initial_position()
        })
    };
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    let mut human = options.human;
    let mut movetime_ms = options.movetime_ms;
    let mut engine_match: Option<EngineMatch> = None;
    if let Some(path) = &options.engine {
        start_engine(&mut engine_match, path, human, movetime_ms, &game);
    }

    let terminal = Terminal::new();
    println!(
        "Terminal commands: ng | ng <fen> | mv e2e4 | mv e7e8q (q/r/b/n) | undo [n] | history | engine <path> | engine off | side white|black | movetime <ms> | exit (or Ctrl-C)"
    );

    loop {
        let plies_before = game.uci_history.len();

        for line in terminal.poll() {
            match terminal::parse_command(&line) {
                Ok(Command::Exit) => return,
                Ok(Command::Engine(None)) => {
                    engine_match = None;
                    println!("Engine off");
                }
                Ok(Command::Engine(Some(path))) => {
                    start_engine(&mut engine_match, &path, human, movetime_ms, &game);
                }
                Ok(Command::Side(side)) => {
                    human = side;
                    if let Some(engine) = engine_match.as_mut() {
                        engine.set_human(side);
                    }
                    println!("You play {side:?}");
                }
                Ok(Command::MoveTime(ms)) => {
                    movetime_ms = ms;
                    if let Some(engine) = engine_match.as_mut() {
                        engine.movetime_ms = ms;
                    }
                    println!("Engine thinks {ms} ms per move");
                }
                _ if engine_match
                    .as_ref()
                    .is_some_and(|engine| engine.blocks_command(&game, &line)) =>
                {
                    eprintln!("It is the engine's turn");
                }
                _ => game.run_command(&line),
            }
        }

        let mut user_inputs = input_handler.poll(&game);
        if engine_match
            .as_ref()
            .is_some_and(|engine| engine.blocks(&game, &user_inputs))
        {
            user_inputs = InputStatus::Chilling;
        }
        game.parse_input(&user_inputs);

        if let Some(engine) = engine_match.as_mut() {
            // Undo and new game rewind the move list: hand the turn back to the human.
            if game.uci_history.len() < plies_before {
                engine.after_undo(&mut game);
            }
            if let Err(e) = engine.tick(&mut game) {
                eprintln!("Engine stopped: {e}");
                engine_match = None;
            }
        }

        renderer
            .run(&game, &user_inputs, input_handler.dialog())
            .await;
    }
}

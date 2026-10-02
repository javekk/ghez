mod render {
    pub mod renderer;
    pub mod theme;
}

mod engine {
    pub mod engine_player;
    pub mod players;
    pub mod tournament;
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

use crate::engine::players::{DEFAULT_MOVETIME_MS, Players};
use crate::engine::tournament::{self, Tournament};
use crate::game::domain::Side;
use crate::game::game::Game;
use crate::inputs::handler::{InputHandler, InputStatus};
use crate::inputs::terminal::{self, Command, Terminal};
use crate::render::renderer::Renderer;

const USAGE: &str = "usage: ghez [--white <human|path>] [--black <human|path>] [--movetime <ms>]
       ghez --match <engineA> <engineB> [--openings <fen|epd file>] [--pairs <n>] [--concurrency <k>] [--movetime <ms>]";
const DEFAULT_CONCURRENCY: usize = 4;

/// Command-line options: `[--white <human|path>] [--black <human|path>] [--movetime <ms>]`.
/// A path hands that colour to the UCI engine at that path; the default is a human.
/// `--match <engineA> <engineB>` plays the two engines against each other instead.
struct Options {
    white: Option<String>,
    black: Option<String>,
    movetime_ms: u64,
    match_engines: Option<(String, String)>,
    openings: Option<String>,
    /// Pairs of games to play; defaults to one per opening.
    pairs: Option<usize>,
    concurrency: usize,
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        white: None,
        black: None,
        movetime_ms: DEFAULT_MOVETIME_MS,
        match_engines: None,
        openings: None,
        pairs: None,
        concurrency: DEFAULT_CONCURRENCY,
    };
    let mut args = args;
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        let engine_path = |v: String| (v != "human").then_some(v);
        match flag.as_str() {
            "--white" => options.white = engine_path(value()?),
            "--black" => options.black = engine_path(value()?),
            "--movetime" => {
                options.movetime_ms = value()?
                    .parse()
                    .map_err(|_| "--movetime expects milliseconds".to_string())?
            }
            "--match" => options.match_engines = Some((value()?, value()?)),
            "--openings" => options.openings = Some(value()?),
            "--pairs" => {
                options.pairs = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--pairs expects a number".to_string())?,
                )
            }
            "--concurrency" => {
                options.concurrency = value()?
                    .parse()
                    .map_err(|_| "--concurrency expects a number".to_string())?
            }
            _ if !flag.starts_with("--") => eprintln!("Ignoring argument '{flag}'"),
            _ => return Err(format!("unknown option '{flag}'")),
        }
    }
    Ok(options)
}

fn set_player(players: &mut Players, side: Side, path: Option<&str>, game: &Game) {
    match players.set(side, path, game) {
        Ok(()) => match path {
            Some(path) => println!("{side:?} is played by the engine {path}"),
            None => println!("{side:?} is played by a human"),
        },
        Err(e) => eprintln!("{e}"),
    }
}

/// Engine A against engine B: no human input, only `exit` on the terminal.
async fn run_match(options: &Options, engine_a: &str, engine_b: &str) {
    let openings = match &options.openings {
        Some(path) => tournament::load_openings(path).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        }),
        None => Vec::new(),
    };
    let pairs = options.pairs.unwrap_or(openings.len().max(1));
    let mut tournament = Tournament::new(
        engine_a,
        engine_b,
        &openings,
        pairs,
        options.concurrency,
        options.movetime_ms,
    );
    println!(
        "Match A={engine_a} vs B={engine_b}: {} games, {} at a time, {} ms per move. Type exit (or Ctrl-C) to quit",
        tournament.total, options.concurrency, options.movetime_ms
    );

    let renderer: Renderer = Renderer::new().await;
    let terminal = Terminal::new();
    let mut reported = false;
    loop {
        for line in terminal.poll() {
            if let Ok(Command::Exit) = terminal::parse_command(&line) {
                return;
            }
        }
        tournament.tick();
        if tournament.is_finished() && !reported {
            reported = true;
            println!("Match over | {}", tournament.summary());
        }
        renderer.run_match(&tournament).await;
    }
}

#[macroquad::main("Ghez")]
async fn main() {
    let options = parse_options(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}\n{USAGE}");
        std::process::exit(2);
    });

    if let Some((engine_a, engine_b)) = &options.match_engines {
        return run_match(&options, engine_a, engine_b).await;
    }

    let fen = "8/8/8/8/8/5k2/4p3/4K3 b - - 0 1";

    let mut game = Game::new_game_from_fen(&fen).unwrap_or_else(|e| {
        eprintln!("Invalid FEN: {e}"); // TODO set a UI status/toast string
        Game::new_game_from_initial_position()
    });
    let renderer: Renderer = Renderer::new().await;
    let mut input_handler: InputHandler = InputHandler::new();

    let mut players = Players::new(options.movetime_ms);
    set_player(&mut players, Side::White, options.white.as_deref(), &game);
    set_player(&mut players, Side::Black, options.black.as_deref(), &game);

    let terminal = Terminal::new();
    println!(
        "Terminal commands: ng | ng <fen> | mv e2e4 | mv e7e8q (q/r/b/n) | undo [n] | history | white <human|path> | black <human|path> | movetime <ms> | exit (or Ctrl-C)"
    );

    loop {
        let plies_before = game.uci_history.len();

        for line in terminal.poll() {
            match terminal::parse_command(&line) {
                Ok(Command::Exit) => return,
                Ok(Command::Player(side, path)) => {
                    set_player(&mut players, side, path.as_deref(), &game);
                }
                Ok(Command::MoveTime(ms)) => {
                    players.movetime_ms = ms;
                    println!("Engines think {ms} ms per move");
                }
                _ if players.blocks_command(&game, &line) => {
                    eprintln!("It is an engine's turn");
                }
                _ => game.run_command(&line),
            }
        }

        let mut user_inputs = input_handler.poll(&game);
        if players.blocks(&game, &user_inputs) {
            user_inputs = InputStatus::Chilling;
        }
        game.parse_input(&user_inputs);

        // Undo and new game rewind the move list: hand the turn back to the human.
        if game.uci_history.len() < plies_before {
            players.after_undo(&mut game);
        }
        players.tick(&mut game);

        renderer
            .run(&game, &user_inputs, input_handler.dialog())
            .await;
    }
}

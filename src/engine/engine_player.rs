use crate::engine::uci::{EngineMessage, UciEngine};
use crate::game::domain::{Move, PieceType, Side};
use crate::game::fen;
use crate::game::game::Game;
use crate::game::game_state::GameState;
use crate::inputs::terminal::{self, Command};

/// The position a search was started from, to tell whether its answer is still valid.
#[derive(Clone, Copy, PartialEq)]
struct Snapshot {
    state: GameState,
    plies: usize,
}

impl Snapshot {
    fn of(game: &Game) -> Self {
        Self {
            state: game.game_state,
            plies: game.uci_history.len(),
        }
    }
}

/// One UCI engine playing one colour of `Game`: owns the process and its search state.
pub struct EnginePlayer {
    engine: UciEngine,
    pub side: Side,
    ready: bool,
    searching: Option<Snapshot>,
    /// A `stop` was sent; the next `bestmove` belongs to the abandoned search.
    draining: bool,
    /// Position where the engine's last answer was unusable, to avoid asking again forever.
    failed_at: Option<Snapshot>,
    /// Starting position of the game the engine was last told about.
    root: GameState,
    quiet: bool,
}

impl EnginePlayer {
    pub fn start(path: &str, side: Side, game: &Game, quiet: bool) -> Result<Self, String> {
        Ok(Self {
            engine: UciEngine::spawn(path, &format!("{side:?}"), quiet)?,
            side,
            ready: false,
            searching: None,
            draining: false,
            failed_at: None,
            root: game.game_history[0],
            quiet,
        })
    }

    /// The engine's last answer was unusable and it has not been asked again since.
    pub fn failed(&self) -> bool {
        self.failed_at.is_some()
    }

    /// Reads engine output, plays its move and starts a new search when it is its turn.
    /// Returns an error message if the engine died.
    pub fn tick(&mut self, game: &mut Game, movetime_ms: u64) -> Result<(), String> {
        for message in self.engine.poll() {
            match message {
                EngineMessage::Name(name) if !self.quiet => {
                    println!("{:?} engine: {name}", self.side)
                }
                EngineMessage::Name(_) => {}
                EngineMessage::UciOk => self.ready = true,
                EngineMessage::BestMove(mv) => self.on_best_move(game, mv),
                EngineMessage::Closed => return Err("engine process exited".to_string()),
            }
        }

        if let Some(snapshot) = self.searching {
            if snapshot != Snapshot::of(game) {
                self.engine.send("stop");
                self.searching = None;
                self.draining = true;
            }
        }

        if game.game_history[0] != self.root {
            self.root = game.game_history[0];
            self.engine.send("ucinewgame");
        }

        let snapshot = Snapshot::of(game);
        let engine_to_move = game.game_state.side == self.side
            && game.pending_promotion.is_none()
            && game.parse_game_status().outcome().is_none();
        if self.ready
            && !self.draining
            && self.searching.is_none()
            && engine_to_move
            && self.failed_at != Some(snapshot)
        {
            self.failed_at = None;
            let moves = game.uci_history.join(" ");
            self.engine.send(&format!(
                "position fen {} moves {moves}",
                fen::to_fen(game.game_history[0])
            ));
            self.engine.send(&format!("go movetime {movetime_ms}"));
            self.searching = Some(snapshot);
        }
        Ok(())
    }

    fn on_best_move(&mut self, game: &mut Game, text: Option<String>) {
        if self.draining {
            self.draining = false;
            return;
        }
        let Some(snapshot) = self.searching.take() else {
            return;
        };
        if snapshot != Snapshot::of(game) {
            return;
        }
        let played = text
            .as_deref()
            .and_then(|text| match terminal::parse_move(text) {
                Ok(Command::Move(from, to, promotion)) => {
                    let piece = game.get_piece(from)?;
                    // Engines always name the promotion piece; default to a queen if one forgets.
                    let mv = Move { piece, from, to };
                    let promotion = promotion.or(mv.is_promotion().then_some(PieceType::Queen));
                    game.request_move(mv, promotion).then_some(())
                }
                _ => None,
            });
        match played {
            Some(()) if self.quiet => {}
            Some(()) => println!("{:?} engine played {}", self.side, text.unwrap_or_default()),
            None => {
                eprintln!("{:?} engine returned no usable move ({text:?})", self.side);
                self.failed_at = Some(snapshot);
            }
        }
    }
}

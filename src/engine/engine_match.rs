use crate::engine::uci::{EngineMessage, UciEngine};
use crate::game::domain::{Move, PieceType, Side};
use crate::game::fen;
use crate::game::game::Game;
use crate::game::game_state::GameState;
use crate::inputs::handler::InputStatus;
use crate::inputs::terminal::{self, Command};

pub const DEFAULT_MOVETIME_MS: u64 = 1000;

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

/// Human vs engine: owns the UCI engine process and plays its side of `Game`.
pub struct EngineMatch {
    engine: UciEngine,
    pub human: Side,
    pub movetime_ms: u64,
    ready: bool,
    searching: Option<Snapshot>,
    /// A `stop` was sent; the next `bestmove` belongs to the abandoned search.
    draining: bool,
    /// Position where the engine's last answer was unusable, to avoid asking again forever.
    failed_at: Option<Snapshot>,
    /// Starting position of the game the engine was last told about.
    root: GameState,
}

impl EngineMatch {
    pub fn start(path: &str, human: Side, movetime_ms: u64, game: &Game) -> Result<Self, String> {
        Ok(Self {
            engine: UciEngine::spawn(path)?,
            human,
            movetime_ms,
            ready: false,
            searching: None,
            draining: false,
            failed_at: None,
            root: game.game_history[0],
        })
    }

    /// Changes the human's colour, abandoning a search made for the other side.
    pub fn set_human(&mut self, human: Side) {
        self.human = human;
        if self.searching.take().is_some() {
            self.engine.send("stop");
            self.draining = true;
        }
    }

    fn engine_side(&self) -> Side {
        self.human.opponent()
    }

    /// True when the input would be the human playing a piece that is not theirs,
    /// or playing while it is the engine's turn.
    pub fn blocks(&self, game: &Game, input: &InputStatus) -> bool {
        let piece_side = match input {
            InputStatus::Dragging(drag) | InputStatus::Releasing(drag, _) => drag.piece.side,
            _ => return false,
        };
        piece_side != self.human || game.game_state.side != self.human
    }

    /// Whether a typed command would move a piece for the engine.
    pub fn blocks_command(&self, game: &Game, line: &str) -> bool {
        matches!(terminal::parse_command(line), Ok(Command::Move(..)))
            && game.game_state.side != self.human
    }

    /// Takes back moves until it is the human's turn again.
    pub fn after_undo(&self, game: &mut Game) {
        if game.game_state.side == self.engine_side() {
            game.undo(1);
        }
    }

    /// Reads engine output, plays its move and starts a new search when it is its turn.
    /// Returns an error message if the engine died.
    pub fn tick(&mut self, game: &mut Game) -> Result<(), String> {
        for message in self.engine.poll() {
            match message {
                EngineMessage::Name(name) => println!("Engine: {name}"),
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
        let engine_to_move = game.game_state.side == self.engine_side()
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
            self.engine
                .send(&format!("go movetime {}", self.movetime_ms));
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
            Some(()) => println!("Engine played {}", text.unwrap_or_default()),
            None => {
                eprintln!("Engine returned no usable move ({text:?})");
                self.failed_at = Some(snapshot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::domain::Square;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// Writing a script while another test forks leaks its fd into the child (ETXTBSY).
    static SPAWN_LOCK: Mutex<()> = Mutex::new(());

    fn start(tag: &str, human: Side, game: &Game) -> EngineMatch {
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        EngineMatch::start(&fake_engine(tag), human, 50, game).unwrap()
    }

    /// A shell "engine" that always answers e7e5, enough to drive one exchange.
    fn fake_engine(tag: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("ghez-fake-engine-{}-{tag}.sh", std::process::id()));
        std::fs::write(
            &path,
            "#!/bin/sh\nwhile read cmd rest; do\n case \"$cmd\" in\n uci) echo 'id name Fake'; echo uciok;;\n go) echo 'bestmove e7e5';;\n quit) exit 0;;\n esac\ndone\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn tick_until(m: &mut EngineMatch, game: &mut Game, done: impl Fn(&Game) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done(game) {
            assert!(Instant::now() < deadline, "engine did not answer in time");
            m.tick(game).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn engine_answers_and_undo_returns_the_turn() {
        let mut game = Game::new_game_from_initial_position();
        let mut m = start("white", Side::White, &game);

        let pawn = game.get_piece(Square::E2).unwrap();
        assert!(game.request_move(
            Move {
                piece: pawn,
                from: Square::E2,
                to: Square::E4
            },
            None
        ));
        tick_until(&mut m, &mut game, |g| g.uci_history.len() == 2);
        assert_eq!(game.uci_history, ["e2e4", "e7e5"]);
        assert_eq!(game.game_state.side, Side::White);

        // Undoing only the engine's move would leave it on turn: take back both plies.
        game.undo(1);
        m.after_undo(&mut game);
        assert!(game.uci_history.is_empty());
    }

    #[test]
    fn engine_moves_first_when_human_is_black() {
        let mut game = Game::new_game_from_initial_position();
        // The fake engine only knows black's reply, so check that a search starts for white.
        let mut m = start("black", Side::Black, &game);
        let deadline = Instant::now() + Duration::from_secs(5);
        while m.searching.is_none() {
            assert!(Instant::now() < deadline, "no search started");
            m.tick(&mut game).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

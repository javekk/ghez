use crate::engine::engine_player::EnginePlayer;
use crate::game::domain::Side;
use crate::game::game::Game;
use crate::inputs::handler::InputStatus;
use crate::inputs::terminal::{self, Command};

pub const DEFAULT_MOVETIME_MS: u64 = 1000;

/// Who plays a colour.
pub enum Controller {
    Human,
    Engine(EnginePlayer),
}

/// Who plays each colour: human vs human, human vs engine or engine vs engine.
pub struct Players {
    white: Controller,
    black: Controller,
    pub movetime_ms: u64,
}

impl Players {
    pub fn new(movetime_ms: u64) -> Self {
        Self {
            white: Controller::Human,
            black: Controller::Human,
            movetime_ms,
        }
    }

    fn controller(&self, side: Side) -> &Controller {
        match side {
            Side::White => &self.white,
            Side::Black => &self.black,
        }
    }

    fn controller_mut(&mut self, side: Side) -> &mut Controller {
        match side {
            Side::White => &mut self.white,
            Side::Black => &mut self.black,
        }
    }

    pub fn is_human(&self, side: Side) -> bool {
        matches!(self.controller(side), Controller::Human)
    }

    /// Hands `side` to the UCI engine at `path`, or back to the human when `None`.
    pub fn set(&mut self, side: Side, path: Option<&str>, game: &Game) -> Result<(), String> {
        // Stop the previous engine first
        *self.controller_mut(side) = Controller::Human;
        if let Some(path) = path {
            let engine = EnginePlayer::start(path, side, game)?;
            *self.controller_mut(side) = Controller::Engine(engine);
        }
        Ok(())
    }

    /// True when the input would move a piece for the wrong colour, or for a colour
    /// the human does not control.
    pub fn blocks(&self, game: &Game, input: &InputStatus) -> bool {
        match input {
            InputStatus::Dragging(drag) | InputStatus::Releasing(drag, _) => {
                drag.piece.side != game.game_state.side || !self.is_human(drag.piece.side)
            }
            _ => false,
        }
    }

    /// Whether a typed command would move a piece for an engine.
    pub fn blocks_command(&self, game: &Game, line: &str) -> bool {
        matches!(terminal::parse_command(line), Ok(Command::Move(..)))
            && !self.is_human(game.game_state.side)
    }

    /// Against a human, takes back one more move so it is the human's turn again.
    pub fn after_undo(&self, game: &mut Game) {
        let side = game.game_state.side;
        if !self.is_human(side) && self.is_human(side.opponent()) {
            game.undo(1);
        }
    }

    /// Lets each engine read its output, play and start searching.
    /// An engine that dies is replaced by a human.
    pub fn tick(&mut self, game: &mut Game) {
        for side in [Side::White, Side::Black] {
            let movetime_ms = self.movetime_ms;
            if let Controller::Engine(engine) = self.controller_mut(side) {
                if let Err(e) = engine.tick(game, movetime_ms) {
                    eprintln!("{side:?} engine stopped: {e}; {side:?} is now played by a human");
                    *self.controller_mut(side) = Controller::Human;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::domain::{Move, Square};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// Writing a script while another test forks leaks its fd into the child (ETXTBSY).
    static SPAWN_LOCK: Mutex<()> = Mutex::new(());

    /// A shell "engine" that answers every `go` with `reply`.
    fn fake_engine(tag: &str, reply: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("ghez-fake-engine-{}-{tag}.sh", std::process::id()));
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\nwhile read cmd rest; do\n case \"$cmd\" in\n uci) echo 'id name Fake'; echo uciok;;\n go) echo 'bestmove {reply}';;\n quit) exit 0;;\n esac\ndone\n"
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn set_fake(players: &mut Players, side: Side, tag: &str, reply: &str, game: &Game) {
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        players
            .set(side, Some(&fake_engine(tag, reply)), game)
            .unwrap();
    }

    fn tick_until(players: &mut Players, game: &mut Game, done: impl Fn(&Game) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done(game) {
            assert!(Instant::now() < deadline, "engine did not answer in time");
            players.tick(game);
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn engine_answers_and_undo_returns_the_turn() {
        let mut game = Game::new_game_from_initial_position();
        let mut players = Players::new(50);
        set_fake(&mut players, Side::Black, "answer", "e7e5", &game);

        let pawn = game.get_piece(Square::E2).unwrap();
        let mv = Move {
            piece: pawn,
            from: Square::E2,
            to: Square::E4,
        };
        assert!(game.request_move(mv, None));
        tick_until(&mut players, &mut game, |g| g.uci_history.len() == 2);
        assert_eq!(game.uci_history, ["e2e4", "e7e5"]);
        assert_eq!(game.game_state.side, Side::White);

        // Undoing only the engine's move would leave it on turn: take back both plies.
        game.undo(1);
        players.after_undo(&mut game);
        assert!(game.uci_history.is_empty());
    }

    #[test]
    fn engine_moves_first_when_it_plays_white() {
        let mut game = Game::new_game_from_initial_position();
        let mut players = Players::new(50);
        set_fake(&mut players, Side::White, "first", "e2e4", &game);
        tick_until(&mut players, &mut game, |g| g.uci_history.len() == 1);
        assert_eq!(game.uci_history, ["e2e4"]);
        assert!(players.blocks_command(&Game::new_game_from_initial_position(), "mv e2e4"));
        assert!(!players.blocks_command(&game, "mv e7e5"));
    }

    #[test]
    fn two_engines_play_each_other() {
        let mut game = Game::new_game_from_initial_position();
        let mut players = Players::new(50);
        set_fake(&mut players, Side::White, "eve-white", "e2e4", &game);
        set_fake(&mut players, Side::Black, "eve-black", "e7e5", &game);
        tick_until(&mut players, &mut game, |g| g.uci_history.len() == 2);
        assert_eq!(game.uci_history, ["e2e4", "e7e5"]);
        // No human to hand the turn back to: undo stays a single ply.
        game.undo(1);
        players.after_undo(&mut game);
        assert_eq!(game.uci_history.len(), 1);
    }
}

use std::collections::VecDeque;

use crate::engine::players::Players;
use crate::game::domain::{self, Side};
use crate::game::fen;
use crate::game::game::Game;
use crate::game::game_state::GameStatus;

/// Match tally, seen from engine A.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Score {
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
}

impl Score {
    pub fn games(&self) -> u32 {
        self.wins + self.draws + self.losses
    }

    /// Points of engine A as a percentage of the games played.
    pub fn percent(&self) -> f32 {
        match self.games() {
            0 => 0.,
            games => 100. * (self.wins as f32 + 0.5 * self.draws as f32) / games as f32,
        }
    }
}

/// Reads one opening per line from a FEN or EPD file. Blank lines and `#` comments are
/// skipped; EPD lines (no move counters) get `0 1`.
pub fn load_openings(path: &str) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read '{path}': {e}"))?;
    parse_openings(&text).map_err(|e| format!("{path}: {e}"))
}

fn parse_openings(text: &str) -> Result<Vec<String>, String> {
    let mut openings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let counters = match words.get(4..6) {
            Some(c) if c.iter().all(|word| word.parse::<u32>().is_ok()) => c.join(" "),
            _ => "0 1".to_string(),
        };
        let fen = format!("{} {counters}", words[..words.len().min(4)].join(" "));
        fen::parse(&fen).map_err(|e| format!("line {}: {e}", number + 1))?;
        openings.push(fen);
    }
    Ok(openings)
}

/// A game waiting to be played.
struct Pairing {
    fen: String,
    a_side: Side,
}

/// One game in progress (or the last one played on this board).
struct Slot {
    game: Game,
    players: Players,
    a_side: Side,
    done: bool,
}

/// Engine A against engine B over several boards at once. Every opening is played
/// twice with the colours swapped.
pub struct Tournament {
    pub engine_a: String,
    pub engine_b: String,
    movetime_ms: u64,
    queue: VecDeque<Pairing>,
    slots: Vec<Slot>,
    concurrency: usize,
    pub score: Score,
    pub total: u32,
}

impl Tournament {
    /// Plays `pairs` pairs of games, cycling through `openings` (FENs; the initial
    /// position when empty), `concurrency` games at a time.
    pub fn new(
        engine_a: &str,
        engine_b: &str,
        openings: &[String],
        pairs: usize,
        concurrency: usize,
        movetime_ms: u64,
    ) -> Self {
        let initial = [domain::INITIAL_POSITION.to_string()];
        let openings = if openings.is_empty() { &initial } else { openings };
        let queue: VecDeque<Pairing> = (0..pairs)
            .flat_map(|i| {
                let fen = &openings[i % openings.len()];
                [Side::White, Side::Black].map(|a_side| Pairing {
                    fen: fen.clone(),
                    a_side,
                })
            })
            .collect();
        Self {
            engine_a: engine_a.to_string(),
            engine_b: engine_b.to_string(),
            movetime_ms,
            total: queue.len() as u32,
            queue,
            slots: Vec::new(),
            concurrency: concurrency.max(1),
            score: Score::default(),
        }
    }

    /// "A: +W =D -L (score%) of N games".
    pub fn summary(&self) -> String {
        let Score { wins, draws, losses } = self.score;
        format!(
            "A: +{wins} ={draws} -{losses} ({:.1}%) after {}/{} games",
            self.score.percent(),
            self.score.games(),
            self.total
        )
    }

    pub fn is_finished(&self) -> bool {
        self.queue.is_empty() && self.slots.iter().all(|slot| slot.done)
    }

    /// The game shown on the board: the first slot.
    pub fn shown(&self) -> Option<&Game> {
        self.slots.first().map(|slot| &slot.game)
    }

    /// Advances every running game, scores the finished ones and starts the queued ones.
    pub fn tick(&mut self) {
        for index in 0..self.slots.len() {
            let slot = &mut self.slots[index];
            if slot.done {
                continue;
            }
            slot.players.tick(&mut slot.game);
            let Some(loser) = Self::result(slot) else {
                continue;
            };
            slot.done = true;
            // Stop the engines of the finished game.
            slot.players = Players::new(self.movetime_ms);
            let a_side = slot.a_side;
            match loser {
                None => self.score.draws += 1,
                Some(side) if side == a_side => self.score.losses += 1,
                Some(_) => self.score.wins += 1,
            }
            let result = match loser {
                None => "1/2-1/2",
                Some(Side::White) => "0-1",
                Some(Side::Black) => "1-0",
            };
            println!("A plays {a_side:?}: {result} | {}", self.summary());
        }

        for index in 0..self.concurrency {
            let free = self.slots.get(index).is_none_or(|slot| slot.done);
            if !free {
                continue;
            }
            let Some(pairing) = self.queue.pop_front() else {
                break;
            };
            match self.start(&pairing) {
                Ok(slot) if index < self.slots.len() => self.slots[index] = slot,
                Ok(slot) => self.slots.push(slot),
                Err(e) => {
                    eprintln!("Match aborted: {e}");
                    self.total -= 1 + self.queue.len() as u32;
                    self.queue.clear();
                }
            }
        }
    }

    fn start(&self, pairing: &Pairing) -> Result<Slot, String> {
        let game = Game::new_game_from_fen(&pairing.fen)
            .map_err(|e| format!("invalid opening '{}': {e}", pairing.fen))?;
        let mut players = Players::new(self.movetime_ms);
        players.quiet = true;
        players.set(pairing.a_side, Some(&self.engine_a), &game)?;
        players.set(pairing.a_side.opponent(), Some(&self.engine_b), &game)?;
        Ok(Slot {
            game,
            players,
            a_side: pairing.a_side,
            done: false,
        })
    }

    /// `None` while the game goes on, else the loser (`None` for a draw). An engine
    /// that died or answered with an unusable move loses.
    fn result(slot: &Slot) -> Option<Option<Side>> {
        match slot.game.parse_game_status() {
            GameStatus::Draw(_) => Some(None),
            GameStatus::Mated(loser) | GameStatus::LostOnTime(loser) | GameStatus::RunAway(loser) => {
                Some(Some(loser))
            }
            GameStatus::Chilling | GameStatus::Battling => [Side::White, Side::Black]
                .into_iter()
                .find(|&side| slot.players.forfeited(side))
                .map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::players::tests::{SPAWN_LOCK, fake_engine};
    use std::time::{Duration, Instant};

    #[test]
    fn pair_swaps_colours_and_bad_move_forfeits() {
        // Held for the whole match: its engines are spawned while ticking.
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let a = fake_engine("match-a", "e2e4");
        let b = fake_engine("match-b", "e7e5");
        let mut tournament = Tournament::new(&a, &b, &[], 1, 2, 20);
        assert_eq!(tournament.total, 2);

        let deadline = Instant::now() + Duration::from_secs(10);
        while !tournament.is_finished() {
            assert!(Instant::now() < deadline, "match did not finish in time");
            tournament.tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        // A as White: e2e4 e7e5, then A repeats e2e4 (illegal) and loses.
        // A as Black: B opens with e7e5 (illegal) and loses.
        assert_eq!(
            tournament.score,
            Score {
                wins: 1,
                draws: 0,
                losses: 1
            }
        );
        assert_eq!(tournament.score.percent(), 50.);
    }

    #[test]
    fn openings_accept_fen_and_epd() {
        let text = "# comment\n\nrnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 3 7\n\
                    rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - bm e4; id \"x\";\n";
        assert_eq!(
            parse_openings(text).unwrap(),
            [
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 3 7",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            ]
        );
        assert!(parse_openings("not a fen").unwrap_err().starts_with("line 1"));
    }
}

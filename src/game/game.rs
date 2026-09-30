use crate::game::domain::{self, CastleRights, Move, MoveLog, Piece, PieceType, Side, Square};
use crate::game::draw_checker::DrawChecker;
use crate::game::game_state::DrawReason::{
    self, FiftyMoveRule, InsufficientMaterial, Stalemate, ThreefoldRepetition,
};
use crate::game::game_state::{GameState, GameStatus};
use crate::game::{fen, movegen, notation};
use crate::inputs::handler::InputStatus;
use crate::inputs::terminal::{self, Command};

pub struct Game {
    pub game_state: GameState,
    pub game_history: Vec<GameState>,
    /// SAN of each played move; `san_history[i]` leads to `game_history[i + 1]`.
    pub san_history: Vec<String>,
    /// Pawn move waiting for the player to pick the promotion piece.
    pub pending_promotion: Option<Move>,
}

impl Game {
    pub fn new() -> Self {
        let game_state = GameState::new();
        let mut game_history = Vec::new();
        game_history.push(game_state);

        Self {
            game_state,
            game_history,
            san_history: Vec::new(),
            pending_promotion: None,
        }
    }

    pub fn new_game_from_fen(fen: &str) -> Result<Self, String> {
        let game_state = fen::parse(fen)?;
        let mut game_history = Vec::new();
        game_history.push(game_state);

        Ok(Self {
            game_state,
            game_history,
            san_history: Vec::new(),
            pending_promotion: None,
        })
    }

    pub fn new_game_from_initial_position() -> Self {
        let game_state = fen::parse(domain::INITIAL_POSITION).unwrap();
        let mut game_history = Vec::new();
        game_history.push(game_state);
        Self {
            game_state,
            game_history,
            san_history: Vec::new(),
            pending_promotion: None,
        }
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        self.game_state.get_piece(square)
    }

    pub fn parse_input(&mut self, input_status: &InputStatus) {
        // TODO add other user actions like reset game or I don't know
        match input_status {
            InputStatus::Chilling => {}
            InputStatus::Dragging(drag) => {
                //println!(
                //    "Dragging {:?} from {:?}, now on: {:?}",
                //    drag.piece, drag.from, drag.mouse_pos
                //);
            }
            InputStatus::Releasing(drag, Some(square)) if drag.from != *square => {
                let mv = Move {
                    piece: drag.piece,
                    from: drag.from,
                    to: *square,
                };
                if self.request_move(mv, None) {
                    println!("FEN: {:?}", fen::to_fen(self.game_state))
                }
            }
            InputStatus::Promoting(Some(kind)) => {
                if let Some(mv) = self.pending_promotion.take() {
                    self.make_move_promoting(mv, *kind);
                    println!("FEN: {:?}", fen::to_fen(self.game_state))
                }
            }
            InputStatus::Promoting(None) => self.pending_promotion = None,
            InputStatus::Undo => {
                self.undo(1);
            }
            InputStatus::Releasing(..) => {}
            InputStatus::FiringNewGame(fen) => {
                *self = match fen {
                    Some(f) => {
                        Game::new_game_from_fen(f).unwrap_or_else(|e| {
                            eprintln!("Invalid FEN: {e}"); // TODO set a UI status/toast string
                            Game::new_game_from_initial_position()
                        })
                    }
                    None => Game::new_game_from_fen(domain::INITIAL_POSITION).unwrap(),
                };
            }
        }
    }

    /// Runs a terminal command line, reporting the outcome on stdout/stderr.
    pub fn run_command(&mut self, line: &str) {
        match terminal::parse_command(line) {
            Ok(Command::NewGame(fen)) => {
                let new_game = match fen {
                    Some(f) => Game::new_game_from_fen(&f),
                    None => Ok(Game::new_game_from_initial_position()),
                };
                match new_game {
                    Ok(g) => {
                        *self = g;
                        println!("New game: {}", fen::to_fen(self.game_state));
                    }
                    Err(e) => eprintln!("Invalid FEN: {e}"),
                }
            }
            Ok(Command::Move(from, to, promotion)) => {
                let Some(piece) = self.get_piece(from) else {
                    eprintln!("Illegal move {from}{to}: no piece on {from}");
                    return;
                };
                self.pending_promotion = None;
                let mv = Move { piece, from, to };
                if self.request_move(mv, promotion) {
                    if self.pending_promotion.is_some() {
                        println!(
                            "Choose the promotion piece in the dialog (or use e.g. mv {from}{to}q)"
                        );
                    } else {
                        println!("Moved {from}{to}. FEN: {}", fen::to_fen(self.game_state));
                    }
                } else {
                    eprintln!("Illegal move {from}{to}");
                }
            }
            Ok(Command::Undo(n)) => {
                let undone = self.undo(n);
                if undone == 0 {
                    eprintln!("Nothing to undo");
                } else {
                    println!(
                        "Undid {undone} move(s). FEN: {}",
                        fen::to_fen(self.game_state)
                    );
                }
            }
            Ok(Command::History) => {
                let rows = self.move_rows();
                if rows.is_empty() {
                    println!("No moves played yet");
                } else {
                    println!("{}", rows.join("\n"));
                }
            }
            Ok(Command::Exit) => {} // handled by the main loop
            Err(e) => eprintln!("{e}"),
        }
    }

    /// Takes back up to `count` moves (and any promotion in progress).
    /// Returns how many moves were actually undone.
    pub fn undo(&mut self, count: usize) -> usize {
        self.pending_promotion = None;
        let n = count.min(self.san_history.len());
        self.san_history.truncate(self.san_history.len() - n);
        self.game_history.truncate(self.game_history.len() - n);
        if let Some(state) = self.game_history.last() {
            self.game_state = *state;
        }
        n
    }

    /// Numbered rows of the moves played so far, e.g. "1. e4 e5".
    pub fn move_rows(&self) -> Vec<String> {
        notation::rows(&self.game_history[0], &self.san_history)
    }

    pub fn get_legal_moves(&self, piece: Piece, from: Square) -> Vec<Square> {
        movegen::get_legal_moves(&self.game_state, piece, from)
    }

    fn is_move_legal(&self, mv: Move) -> bool {
        movegen::is_move_legal(&self.game_state, mv)
    }

    pub fn parse_game_status(&self) -> GameStatus {
        if movegen::is_mate(&self.game_state) {
            return GameStatus::Mated(self.game_state.side);
        }

        if let Some(draw_reason) = DrawChecker::draw_reason(self) {
            return GameStatus::Draw(draw_reason);
        }

        return GameStatus::Chilling;
    }

    /// Plays `mv`. A promotion without a chosen piece is parked in
    /// `pending_promotion` until the player picks one. Returns false if the
    /// move is illegal.
    fn request_move(&mut self, mv: Move, promotion: Option<PieceType>) -> bool {
        if !mv.is_promotion() {
            return self.make_move(mv);
        }
        if !self.get_legal_moves(mv.piece, mv.from).contains(&mv.to) {
            return false;
        }
        match promotion {
            Some(kind) => self.make_move_promoting(mv, kind),
            None => {
                self.pending_promotion = Some(mv);
                true
            }
        }
    }

    /// Plays `mv`, promoting to a queen if it is a promotion.
    pub fn make_move(&mut self, mv: Move) -> bool {
        self.make_move_promoting(mv, PieceType::Queen)
    }

    pub fn make_move_promoting(&mut self, mv: Move, promote_to: PieceType) -> bool {
        if mv.is_promotion()
            && !matches!(
                promote_to,
                PieceType::Queen | PieceType::Rook | PieceType::Bishop | PieceType::Knight
            )
        {
            return false;
        }
        if !self.get_legal_moves(mv.piece, mv.from).contains(&mv.to) {
            return false;
        }

        let san = notation::san_base(&self.game_state, mv, promote_to);
        let captured_piece = self.game_state.get_piece(mv.to);
        let is_en_passant = self.is_en_passant_capture(mv, captured_piece);

        self.game_state.move_piece(mv.from, mv.to);

        if is_en_passant {
            self.remove_en_passant_captured_pawn(mv);
        }
        self.update_en_passant_target(mv);
        self.update_castle_rights(mv, captured_piece);
        self.relocate_rook_on_castle(mv);

        self.make_pawn_promotion(mv, promote_to);

        self.game_state.side = self.game_state.side.opponent();

        if mv.piece.side == Side::Black {
            self.game_state.fullmove_number = self.game_state.fullmove_number + 1;
        }

        if captured_piece.is_some() || mv.piece.kind == PieceType::Pawn {
            self.game_state.halfmove_counter = 0;
        } else {
            self.game_state.halfmove_counter = self.game_state.halfmove_counter + 1;
        }

        // Save previuos game state
        self.game_history.push(self.game_state.clone());
        self.san_history
            .push(format!("{san}{}", notation::suffix(&self.game_state)));

        true
    }

    fn is_en_passant_capture(&self, mv: Move, captured_piece: Option<Piece>) -> bool {
        mv.piece.kind == PieceType::Pawn
            && self.game_state.en_passant == Some(mv.to)
            && captured_piece.is_none()
    }

    fn remove_en_passant_captured_pawn(&mut self, mv: Move) {
        // The captured pawn sits beside the destination: same file as `to`, same
        // rank as `from`.
        if let Some(captured_square) = Square::from_file_rank(mv.to.file(), mv.from.rank()) {
            self.game_state.clear_square(captured_square);
        }
    }

    fn update_en_passant_target(&mut self, mv: Move) {
        self.game_state.en_passant = if mv.is_pawn_double_push() {
            let behind = mv.from.rank() + mv.piece.side.direction();
            Square::from_file_rank(mv.from.file(), behind)
        } else {
            None
        };
    }

    fn update_castle_rights(&mut self, mv: Move, captured_piece: Option<Piece>) {
        let rights = &mut self.game_state.available_castle;

        if mv.piece.kind == PieceType::King {
            match mv.piece.side {
                Side::White => {
                    rights.white_kingside = false;
                    rights.white_queenside = false;
                }
                Side::Black => {
                    rights.black_kingside = false;
                    rights.black_queenside = false;
                }
            }
        }

        if mv.piece.kind == PieceType::Rook {
            revoke_right_for_rook_square(rights, mv.piece.side, mv.from);
        }

        if let Some(captured) = captured_piece {
            if captured.kind == PieceType::Rook {
                revoke_right_for_rook_square(rights, captured.side, mv.to);
            }
        }
    }

    fn relocate_rook_on_castle(&mut self, mv: Move) {
        if !mv.is_castle() {
            return;
        }

        let (rook_from, rook_to) = match (mv.piece.side, mv.to) {
            (Side::White, Square::G1) => (Square::H1, Square::F1),
            (Side::White, Square::C1) => (Square::A1, Square::D1),
            (Side::Black, Square::G8) => (Square::H8, Square::F8),
            (Side::Black, Square::C8) => (Square::A8, Square::D8),
            _ => return,
        };

        debug_assert!(self.game_state.move_piece(rook_from, rook_to));
    }

    fn make_pawn_promotion(&mut self, mv: Move, promote_to: PieceType) {
        if mv.piece.kind == PieceType::Pawn {
            match (mv.piece.side, mv.to.rank()) {
                (Side::White, 7) => self.game_state.set_piece(
                    mv.to,
                    Piece {
                        side: Side::White,
                        kind: promote_to,
                    },
                ),
                (Side::Black, 0) => self.game_state.set_piece(
                    mv.to,
                    Piece {
                        side: Side::Black,
                        kind: promote_to,
                    },
                ),
                _ => return,
            }
        }
    }

    fn get_squares_under_attacks(&self) -> Vec<Square> {
        Square::ALL
            .into_iter()
            .filter(|&square| self.is_square_under_attack(square))
            .collect()
    }

    fn is_square_under_attack(&self, square: Square) -> bool {
        movegen::is_square_attacked(&self.game_state, square)
    }

    fn get_pseudo_legal_moves(&self, piece: Piece, from: Square) -> Vec<Square> {
        movegen::pseudo_legal_moves(&self.game_state, piece, from)
    }
}

fn revoke_right_for_rook_square(rights: &mut CastleRights, side: Side, square: Square) {
    match (side, square) {
        (Side::White, Square::A1) => rights.white_queenside = false,
        (Side::White, Square::H1) => rights.white_kingside = false,
        (Side::Black, Square::A8) => rights.black_queenside = false,
        (Side::Black, Square::H8) => rights.black_kingside = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn play(game: &mut Game, moves: &[&str]) {
        for m in moves {
            game.run_command(&format!("mv {m}"));
        }
    }

    #[test]
    fn records_san_history_and_rows() {
        let mut game = Game::new_game_from_initial_position();
        play(&mut game, &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"]);
        assert_eq!(game.san_history, ["e4", "e5", "Nf3", "Nc6", "Bb5"]);
        assert_eq!(game.move_rows(), ["1. e4 e5", "2. Nf3 Nc6", "3. Bb5"]);
    }

    #[test]
    fn san_handles_captures_castling_promotion_and_mate() {
        let mut game = Game::new_game_from_initial_position();
        play(&mut game, &["e2e4", "d7d5", "e4d5"]);
        assert_eq!(game.san_history[2], "exd5");

        let mut game = Game::new_game_from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        play(&mut game, &["e1g1", "e8c8"]);
        assert_eq!(game.san_history, ["O-O", "O-O-O"]);

        let mut game = Game::new_game_from_fen("8/4P3/8/8/8/8/k7/4K3 w - - 0 1").unwrap();
        play(&mut game, &["e7e8n"]);
        assert_eq!(game.san_history, ["e8=N"]);

        let mut game = Game::new_game_from_initial_position();
        play(&mut game, &["f2f3", "e7e5", "g2g4", "d8h4"]);
        assert_eq!(game.san_history[3], "Qh4#");
    }

    #[test]
    fn san_disambiguates_same_piece_moves() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/1K6/R6R w - - 0 1").unwrap();
        play(&mut game, &["a1d1"]);
        assert_eq!(game.san_history, ["Rad1"]);
    }

    #[test]
    fn undo_restores_previous_position() {
        let mut game = Game::new_game_from_initial_position();
        let start = fen::to_fen(game.game_state);
        play(&mut game, &["e2e4", "e7e5"]);
        let after_e4 = {
            let mut g = Game::new_game_from_initial_position();
            play(&mut g, &["e2e4"]);
            fen::to_fen(g.game_state)
        };
        assert_eq!(game.undo(1), 1);
        assert_eq!(fen::to_fen(game.game_state), after_e4);
        assert_eq!(game.san_history, ["e4"]);
        game.run_command("undo 5");
        assert_eq!(fen::to_fen(game.game_state), start);
        assert!(game.san_history.is_empty());
        assert_eq!(game.undo(1), 0);
    }

    #[test]
    fn undo_cancels_pending_promotion() {
        let mut game = Game::new_game_from_fen("8/4P3/8/8/8/8/k7/4K3 w - - 0 1").unwrap();
        game.run_command("mv e7e8");
        assert!(game.pending_promotion.is_some());
        game.undo(1);
        assert!(game.pending_promotion.is_none());
    }

    #[test]
    fn rows_start_with_black_to_move() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K3 b - - 0 7").unwrap();
        play(&mut game, &["e8e7", "e1e2"]);
        assert_eq!(game.move_rows(), ["7... Ke7", "8. Ke2"]);
    }

    #[test]
    fn terminal_commands_move_and_reset() {
        let mut game = Game::new_game_from_initial_position();
        game.run_command("mv e2e4");
        assert!(game.get_piece(Square::E4).is_some());
        let before = fen::to_fen(game.game_state);
        game.run_command("mv e2e5"); // illegal: unchanged
        game.run_command("mv e7e5e5");
        assert_eq!(fen::to_fen(game.game_state), before);
        game.run_command("ng");
        assert!(game.get_piece(Square::E4).is_none());
    }

    fn moves_set(game: &Game, piece: Piece, sq: Square) -> HashSet<Square> {
        game.get_pseudo_legal_moves(piece, sq).into_iter().collect()
    }

    fn white(kind: PieceType) -> Piece {
        Piece {
            side: Side::White,
            kind,
        }
    }

    fn black(kind: PieceType) -> Piece {
        Piece {
            side: Side::Black,
            kind,
        }
    }

    #[test]
    fn fen_starting_position_places_white_back_rank() {
        let game = Game::new_game_from_fen(domain::INITIAL_POSITION).unwrap();
        assert_eq!(
            game.game_state.get_piece(Square::A1),
            Some(Piece {
                side: Side::White,
                kind: PieceType::Rook
            })
        );
        assert_eq!(
            game.game_state.get_piece(Square::E1),
            Some(Piece {
                side: Side::White,
                kind: PieceType::King
            })
        );
    }

    #[test]
    fn fen_starting_position_places_black_back_rank() {
        let game = Game::new_game_from_fen(domain::INITIAL_POSITION).unwrap();
        assert_eq!(
            game.game_state.get_piece(Square::E8),
            Some(Piece {
                side: Side::Black,
                kind: PieceType::King
            })
        );
        assert_eq!(
            game.game_state.get_piece(Square::H8),
            Some(Piece {
                side: Side::Black,
                kind: PieceType::Rook
            })
        );
    }

    #[test]
    fn fen_starting_position_middle_ranks_are_empty() {
        let game = Game::new_game_from_fen(domain::INITIAL_POSITION).unwrap();
        for sq_idx in 16..48 {
            let sq = Square::from_index(sq_idx).unwrap();
            assert!(
                game.game_state.get_piece(sq).is_none(),
                "expected {:?} empty",
                sq
            );
        }
    }

    #[test]
    fn knight_on_empty_board_has_eight_moves_from_center() {
        let game = Game::new_game_from_fen("8/8/8/8/3N4/8/8/8 w - - 0 1").unwrap();
        let knight = Piece {
            side: Side::White,
            kind: PieceType::Knight,
        };
        let moves = moves_set(&game, knight, Square::D4);
        let expected: HashSet<Square> = [
            Square::B3,
            Square::B5,
            Square::C2,
            Square::C6,
            Square::E2,
            Square::E6,
            Square::F3,
            Square::F5,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn knight_in_corner_has_two_moves() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/N7 w - - 0 1").unwrap();
        let knight = Piece {
            side: Side::White,
            kind: PieceType::Knight,
        };
        let moves = moves_set(&game, knight, Square::A1);
        let expected: HashSet<Square> = [Square::B3, Square::C2].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn knight_is_blocked_by_friendly_piece_and_captures_enemy() {
        // White knight d4, white pawn f5 (blocks), black pawn e6 (capture).
        let game = Game::new_game_from_fen("8/8/4p3/5P2/3N4/8/8/8 w - - 0 1").unwrap();
        let knight = Piece {
            side: Side::White,
            kind: PieceType::Knight,
        };
        let moves = moves_set(&game, knight, Square::D4);
        assert!(!moves.contains(&Square::F5), "blocked by friendly");
        assert!(moves.contains(&Square::E6), "should capture enemy");
    }

    #[test]
    fn white_pawn_on_start_can_single_or_double_step() {
        let game = Game::new_game_from_fen(
            domain::EMPTY_BOARD
                .replace("8/8/8/8/8/8/8/8", "8/8/8/8/8/8/4P3/8")
                .as_str(),
        )
        .unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E2);
        let expected: HashSet<Square> = [Square::E3, Square::E4].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn white_pawn_off_start_only_single_step() {
        let game = Game::new_game_from_fen("8/8/8/8/8/4P3/8/8 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E3);
        assert_eq!(moves, [Square::E4].into_iter().collect());
    }

    #[test]
    fn white_pawn_blocked_cannot_advance() {
        let game = Game::new_game_from_fen("8/8/8/8/8/4p3/4P3/8 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E2);
        assert!(
            moves.is_empty(),
            "blocked pawn has no quiet moves, got {:?}",
            moves
        );
    }

    #[test]
    fn a_file_pawn_does_not_wrap_to_h_file_on_capture() {
        // White pawn on a2; a black piece sits on h2 (would be "left capture" if file wraps).
        let game = Game::new_game_from_fen("8/8/8/8/8/8/P6p/8 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::A2);
        let expected: HashSet<Square> = [Square::A3, Square::A4].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn white_pawn_captures_diagonally() {
        let game = Game::new_game_from_fen("8/8/8/8/3p1p2/4P3/8/8 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E3);
        let expected: HashSet<Square> = [Square::E4, Square::D4, Square::F4].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn white_pawn_does_not_capture_friendly_pieces() {
        let game = Game::new_game_from_fen("8/8/8/8/3p1P2/4P3/8/8 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E3);
        let expected: HashSet<Square> = [Square::E4, Square::D4].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn black_pawn_on_start_can_double_step_downward() {
        let game = Game::new_game_from_fen("8/4p3/8/8/8/8/8/8 b - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::Black,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E7);
        let expected: HashSet<Square> = [Square::E6, Square::E5].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn black_pawn_does_not_capture_friendly_pieces() {
        let game = Game::new_game_from_fen("8/4p3/3PPp2/8/8/8/8/8 b - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::Black,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E7);
        let expected: HashSet<Square> = [Square::D6].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn bishop_can_move_in_all_squares_not_blocked_by_friendly_pieces() {
        let game = Game::new_game_from_fen("8/8/2N5/8/4B3/8/8/N7 w - - 0 1").unwrap();
        let bishop = Piece {
            side: Side::White,
            kind: PieceType::Bishop,
        };
        let moves = moves_set(&game, bishop, Square::E4);
        let expected: HashSet<Square> = [
            Square::D5,
            Square::F5,
            Square::G6,
            Square::H7,
            Square::F3,
            Square::G2,
            Square::H1,
            Square::D3,
            Square::C2,
            Square::B1,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn bishop_can_move_in_all_squares_until_finds_first_eneny_piece() {
        let game = Game::new_game_from_fen("8/1K6/2n5/8/4B3/8/8/N7 w - - 0 1").unwrap();
        let bishop = Piece {
            side: Side::White,
            kind: PieceType::Bishop,
        };
        let moves = moves_set(&game, bishop, Square::E4);
        let expected: HashSet<Square> = [
            Square::D5,
            Square::F5,
            Square::G6,
            Square::H7,
            Square::F3,
            Square::G2,
            Square::H1,
            Square::D3,
            Square::C2,
            Square::B1,
            Square::C6,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn bishop_can_move_in_all_squares_until_finds_first_eneny_piece_or_friendly_piece() {
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/3b4/4k3/8/N7 b - - 0 1").unwrap();
        let bishop = Piece {
            side: Side::Black,
            kind: PieceType::Bishop,
        };
        let moves = moves_set(&game, bishop, Square::D4);
        let expected: HashSet<Square> =
            [Square::C5, Square::B6, Square::C3, Square::B2, Square::A1]
                .into_iter()
                .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn rook_can_move_in_all_squares_until_finds_friendly_piece() {
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4b3/4r3/8/N7 b - - 0 1").unwrap();
        let rook = Piece {
            side: Side::Black,
            kind: PieceType::Rook,
        };
        let moves = moves_set(&game, rook, Square::E3);
        let expected: HashSet<Square> = [
            Square::E2,
            Square::E1,
            Square::F3,
            Square::G3,
            Square::H3,
            Square::D3,
            Square::C3,
            Square::B3,
            Square::A3,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn rook_can_move_in_all_squares_until_finds_first_eneny_piece_or_friendly_piece() {
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/N7 w - - 0 1").unwrap();
        let rook = Piece {
            side: Side::White,
            kind: PieceType::Rook,
        };
        let moves = moves_set(&game, rook, Square::E4);
        let expected: HashSet<Square> = [
            Square::E5,
            Square::E3,
            Square::F4,
            Square::G4,
            Square::H4,
            Square::D4,
            Square::C4,
            Square::B4,
            Square::A4,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn queen_can_move_in_all_squares_until_finds_first_eneny_piece_or_friendly_piece() {
        let game = Game::new_game_from_fen("7p/1K6/1N6/4q3/4R3/4r3/8/N7 b - - 0 1").unwrap();
        let queen = Piece {
            side: Side::Black,
            kind: PieceType::Queen,
        };
        let moves = moves_set(&game, queen, Square::E5);
        let expected: HashSet<Square> = [
            Square::F6,
            Square::G7,
            Square::F4,
            Square::G3,
            Square::H2,
            Square::D4,
            Square::C3,
            Square::B2,
            Square::A1,
            Square::D6,
            Square::C7,
            Square::B8,
            Square::E6,
            Square::E7,
            Square::E8,
            Square::E4,
            Square::F5,
            Square::G5,
            Square::H5,
            Square::D5,
            Square::C5,
            Square::B5,
            Square::A5,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn king_can_move_in_all_squares_but_not_on_friendly_piece() {
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/N7 w - - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::B7);
        let expected: HashSet<Square> = [
            Square::C8,
            Square::C7,
            Square::C6,
            Square::A6,
            Square::A7,
            Square::A8,
            Square::B8,
        ]
        .into_iter()
        .collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn king_can_move_in_all_squares_and_capture_enemy_pieces() {
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/Nk6 b - - 0 1").unwrap();
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::B1);
        let expected: HashSet<Square> =
            [Square::C2, Square::C1, Square::A1, Square::A2, Square::B2]
                .into_iter()
                .collect();
        assert_eq!(moves, expected);
    }

    // attacked squares

    fn to_hash_set<I: IntoIterator<Item = Square>>(squares: I) -> HashSet<Square> {
        squares.into_iter().collect()
    }

    #[test]
    fn squares_are_under_attack_by_white_pawn() {
        let game = Game::new_game_from_fen("8/8/8/8/3P4/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::C5, Square::E5]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_pawns() {
        let game = Game::new_game_from_fen("8/1P6/8/8/8/8/6P1/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::A8, Square::C8, Square::F3, Square::H3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_white_pawns() {
        let game = Game::new_game_from_fen("8/1P6/8/8/8/8/6P1/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_pawn() {
        let game = Game::new_game_from_fen("8/8/8/8/3p4/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::C3, Square::E3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_pawns() {
        let game = Game::new_game_from_fen("8/7p/8/8/8/1p6/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::G6, Square::A2, Square::C2]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_black_pawns() {
        let game = Game::new_game_from_fen("8/7p/8/8/8/1p6/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_knight() {
        let game = Game::new_game_from_fen("8/8/8/4N3/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::D3,
            Square::F3,
            Square::G4,
            Square::G6,
            Square::F7,
            Square::D7,
            Square::C6,
            Square::C4,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_knights() {
        let game = Game::new_game_from_fen("8/8/8/4N3/8/4N3/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::D3,
            Square::F3,
            Square::G4,
            Square::G6,
            Square::F7,
            Square::D7,
            Square::C6,
            Square::C4,
            Square::D5,
            Square::F5,
            Square::C2,
            Square::G2,
            Square::D1,
            Square::F1,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_white_knights() {
        let game = Game::new_game_from_fen("8/8/1N6/8/8/6N1/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_knight() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/7n w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::F2, Square::G3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_black_knight() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/7n b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_king() {
        let game = Game::new_game_from_fen("8/8/8/4K3/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::D4,
            Square::E4,
            Square::F4,
            Square::F5,
            Square::F6,
            Square::E6,
            Square::D6,
            Square::D5,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_white_king() {
        let game = Game::new_game_from_fen("8/8/8/4K3/8/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_king() {
        let game = Game::new_game_from_fen("k7/8/8/4K3/8/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::A7, Square::B7, Square::B8]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_bishop() {
        let game = Game::new_game_from_fen("8/8/8/3B4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A2,
            Square::B3,
            Square::C4,
            Square::E6,
            Square::F7,
            Square::G8,
            Square::A8,
            Square::B7,
            Square::C6,
            Square::E4,
            Square::F3,
            Square::G2,
            Square::H1,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_bishop_ray_blocked_by_friendly_piece() {
        let game = Game::new_game_from_fen("8/5P2/8/3B4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A2,
            Square::B3,
            Square::C4,
            Square::E6,
            Square::F7,
            Square::A8,
            Square::B7,
            Square::C6,
            Square::E4,
            Square::F3,
            Square::G2,
            Square::H1,
            Square::E8,
            Square::G8,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_bishop_ray_blocked_by_enemy_piece() {
        let game = Game::new_game_from_fen("8/5p2/8/3B4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A2,
            Square::B3,
            Square::C4,
            Square::E6,
            Square::F7,
            Square::A8,
            Square::B7,
            Square::C6,
            Square::E4,
            Square::F3,
            Square::G2,
            Square::H1,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_bishop() {
        let game = Game::new_game_from_fen("8/8/8/3b4/8/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A2,
            Square::B3,
            Square::C4,
            Square::E6,
            Square::F7,
            Square::G8,
            Square::A8,
            Square::B7,
            Square::C6,
            Square::E4,
            Square::F3,
            Square::G2,
            Square::H1,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_white_bishop_wrong_side_to_move() {
        let game = Game::new_game_from_fen("8/8/8/3B4/8/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_rook() {
        let game = Game::new_game_from_fen("8/8/8/3R4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A5,
            Square::B5,
            Square::C5,
            Square::E5,
            Square::F5,
            Square::G5,
            Square::H5,
            Square::D1,
            Square::D2,
            Square::D3,
            Square::D4,
            Square::D6,
            Square::D7,
            Square::D8,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_rook_ray_blocked_by_friendly_piece() {
        let game = Game::new_game_from_fen("8/3P4/8/3R4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A5,
            Square::B5,
            Square::C5,
            Square::E5,
            Square::F5,
            Square::G5,
            Square::H5,
            Square::D1,
            Square::D2,
            Square::D3,
            Square::D4,
            Square::D6,
            Square::D7,
            Square::C8,
            Square::E8,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_rook_ray_blocked_by_enemy_piece() {
        let game = Game::new_game_from_fen("8/3p4/8/3R4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A5,
            Square::B5,
            Square::C5,
            Square::E5,
            Square::F5,
            Square::G5,
            Square::H5,
            Square::D1,
            Square::D2,
            Square::D3,
            Square::D4,
            Square::D6,
            Square::D7,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_rook_in_corner() {
        let game = Game::new_game_from_fen("r7/8/8/8/8/8/8/8 w - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A1,
            Square::A2,
            Square::A3,
            Square::A4,
            Square::A5,
            Square::A6,
            Square::A7,
            Square::B8,
            Square::C8,
            Square::D8,
            Square::E8,
            Square::F8,
            Square::G8,
            Square::H8,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_queen() {
        let game = Game::new_game_from_fen("8/8/8/3Q4/8/8/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::A5,
            Square::B5,
            Square::C5,
            Square::E5,
            Square::F5,
            Square::G5,
            Square::H5,
            Square::D1,
            Square::D2,
            Square::D3,
            Square::D4,
            Square::D6,
            Square::D7,
            Square::D8,
            Square::A2,
            Square::B3,
            Square::C4,
            Square::E6,
            Square::F7,
            Square::G8,
            Square::A8,
            Square::B7,
            Square::C6,
            Square::E4,
            Square::F3,
            Square::G2,
            Square::H1,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_king_can_castle_kingside_when_path_clear() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_without_rights() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w - - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_path_occupied() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4KB1R w K - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_transit_attacked() {
        let game = Game::new_game_from_fen("4kr2/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_destination_attacked() {
        let game = Game::new_game_from_fen("4k1r1/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_when_in_check() {
        let game = Game::new_game_from_fen("4k3/4r3/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_can_castle_queenside_when_path_clear() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(moves.contains(&Square::C1));
    }

    #[test]
    fn white_king_cannot_castle_queenside_when_b1_occupied() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/RN2K3 w Q - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::C1));
    }

    #[test]
    fn white_king_cannot_castle_queenside_when_d1_attacked() {
        let game = Game::new_game_from_fen("3rk3/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::C1));
    }

    #[test]
    fn black_king_can_castle_both_sides() {
        let game = Game::new_game_from_fen("r3k2r/8/8/8/8/8/8/4K3 b kq - 0 1").unwrap();
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E8);
        assert!(moves.contains(&Square::G8));
        assert!(moves.contains(&Square::C8));
    }

    #[test]
    fn black_king_cannot_castle_kingside_when_f8_attacked_by_white_bishop() {
        let game = Game::new_game_from_fen("4k2r/8/8/8/8/B7/8/4K3 b k - 0 1").unwrap();
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E8);
        assert!(!moves.contains(&Square::G8));
    }

    #[test]
    fn white_queen_blocked_on_all_rays() {
        let game = Game::new_game_from_fen("8/8/8/2PPP3/2PQP3/2PPP3/8/8 b - - 0 1").unwrap();
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([
            Square::C3,
            Square::D3,
            Square::E3,
            Square::C4,
            Square::E4,
            Square::C5,
            Square::D5,
            Square::E5,
            Square::B4,
            Square::D4,
            Square::F4,
            Square::B5,
            Square::F5,
            Square::B6,
            Square::C6,
            Square::D6,
            Square::E6,
            Square::F6,
        ]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn white_pawn_on_start_has_two_forward_moves() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(pawn, Square::E2));
        let expected: HashSet<Square> = to_hash_set([Square::E3, Square::E4]);
        assert_eq!(legal, expected);
    }

    #[test]
    fn pinned_pawn_cannot_move_off_the_pin_ray() {
        let game = Game::new_game_from_fen("k3r3/8/8/8/6b1/8/4P3/3K4 w - - 0 1").unwrap();
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let legal = game.get_legal_moves(pawn, Square::E2);
        assert!(
            legal.is_empty(),
            "pinned pawn should have no legal moves, got {:?}",
            legal
        );
    }

    #[test]
    fn white_king_can_castle_king_side() {
        let game = Game::new_game_from_fen(
            "rnbqkb1r/ppp2ppp/3ppn2/8/8/3BPN2/PPPP1PPP/RNBQK2R w KQkq - 0 1",
        )
        .unwrap();
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(king, Square::E1));
        let expected: HashSet<Square> = to_hash_set([Square::E2, Square::F1, Square::G1]);
        assert_eq!(legal, expected);
    }

    #[test]
    fn black_king_can_castle_king_side() {
        let game = Game::new_game_from_fen(
            "rnbqk2r/ppp1bppp/3ppn2/8/8/1P1BPN2/P1PP1PPP/RNBQK2R b KQkq - 0 1",
        )
        .unwrap();
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(king, Square::E8));
        let expected: HashSet<Square> = to_hash_set([Square::D7, Square::F8, Square::G8]);
        assert_eq!(legal, expected);
    }

    #[test]
    fn black_pawn_can_take_en_passant() {
        let game = Game::new_game_from_fen(
            "r1bqkbnr/1ppp1ppp/2n5/1B2p3/pP2P3/3P1N2/P1P2PPP/RNBQK2R b KQkq b3 0 5",
        )
        .unwrap();
        let pawn = Piece {
            side: Side::Black,
            kind: PieceType::Pawn,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(pawn, Square::A4));
        let expected: HashSet<Square> = to_hash_set([Square::A3, Square::B3]);
        assert_eq!(legal, expected);
    }

    #[test]
    fn black_cannot_castle_queenside() {
        let game =
            Game::new_game_from_fen("r3k3/pppnqNpp/8/4p3/Q1Bn2b1/2P5/PP1P1KPP/RNB4R b q - 0 13")
                .unwrap();
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(king, Square::E8));
        let expected: HashSet<Square> = to_hash_set([Square::F8]);
        assert_eq!(legal, expected);
    }

    // region: fen parsing

    #[test]
    fn fen_parses_side_to_move() {
        assert_eq!(
            Game::new_game_from_fen("8/8/8/8/8/8/8/8 w - - 0 1")
                .unwrap()
                .game_state
                .side,
            Side::White
        );
        assert_eq!(
            Game::new_game_from_fen("8/8/8/8/8/8/8/8 b - - 0 1")
                .unwrap()
                .game_state
                .side,
            Side::Black
        );
    }

    #[test]
    fn fen_parses_all_castle_rights() {
        let rights = Game::new_game_from_fen(domain::INITIAL_POSITION)
            .unwrap()
            .game_state
            .available_castle;
        assert!(rights.white_kingside);
        assert!(rights.white_queenside);
        assert!(rights.black_kingside);
        assert!(rights.black_queenside);
    }

    #[test]
    fn fen_parses_partial_castle_rights() {
        let rights = Game::new_game_from_fen("8/8/8/8/8/8/8/8 w Kq - 0 1")
            .unwrap()
            .game_state
            .available_castle;
        assert!(rights.white_kingside);
        assert!(!rights.white_queenside);
        assert!(!rights.black_kingside);
        assert!(rights.black_queenside);
    }

    #[test]
    fn fen_parses_no_castle_rights() {
        let rights = Game::new_game_from_fen("8/8/8/8/8/8/8/8 w - - 0 1")
            .unwrap()
            .game_state
            .available_castle;
        assert!(!rights.is_castle_still_available());
    }

    #[test]
    fn fen_parses_en_passant_square() {
        assert_eq!(
            Game::new_game_from_fen("8/8/8/8/8/8/8/8 w - e3 0 1")
                .unwrap()
                .game_state
                .en_passant,
            Some(Square::E3)
        );
    }

    #[test]
    fn fen_parses_no_en_passant_square() {
        assert_eq!(
            Game::new_game_from_fen("8/8/8/8/8/8/8/8 w - - 0 1")
                .unwrap()
                .game_state
                .en_passant,
            None
        );
    }

    #[test]
    fn fen_empty_board_has_no_pieces() {
        let game = Game::new_game_from_fen(domain::EMPTY_BOARD).unwrap();
        for i in 0..64 {
            assert!(
                game.game_state
                    .get_piece(Square::from_index(i).unwrap())
                    .is_none()
            );
        }
    }

    #[test]
    fn fen_places_all_piece_types_on_correct_squares() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/RNBQKBNR w - - 0 1").unwrap();
        let expected = [
            (Square::A1, PieceType::Rook),
            (Square::B1, PieceType::Knight),
            (Square::C1, PieceType::Bishop),
            (Square::D1, PieceType::Queen),
            (Square::E1, PieceType::King),
            (Square::F1, PieceType::Bishop),
            (Square::G1, PieceType::Knight),
            (Square::H1, PieceType::Rook),
        ];
        for (sq, kind) in expected {
            assert_eq!(game.game_state.get_piece(sq), Some(white(kind)));
        }
    }

    // endregion

    // region: get_pseudo_legal_moves dispatch

    #[test]
    fn pseudo_legal_dispatch_matches_each_piece_kind() {
        // Lone piece in the center of an empty board; assert the dispatch routes
        // to a non-empty result per kind (specifics covered elsewhere).
        for kind in [
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ] {
            let game = Game::new_game_from_fen("8/8/8/8/3Q4/8/8/8 w - - 0 1").unwrap();
            let moves = game.get_pseudo_legal_moves(white(kind), Square::D4);
            assert!(
                !moves.is_empty(),
                "{:?} should have pseudo-legal moves",
                kind
            );
        }
    }

    // endregion

    // region: get_legal_moves en passant

    #[test]
    fn get_legal_moves_includes_en_passant_capture() {
        let game = Game::new_game_from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let legal: HashSet<Square> =
            to_hash_set(game.get_legal_moves(white(PieceType::Pawn), Square::E5));
        assert!(legal.contains(&Square::D6));
    }

    #[test]
    fn get_legal_moves_empty_for_wrong_side_piece() {
        let game = Game::new_game_from_fen("4k3/4p3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(
            game.get_legal_moves(black(PieceType::Pawn), Square::E7)
                .is_empty()
        );
    }

    // endregion

    // region: full move sequences

    #[test]
    fn en_passant_available_immediately_after_double_push() {
        // Black double-pushes d7-d5, then white can capture e5xd6 e.p. next move.
        let mut game = Game::new_game_from_fen("4k3/3p4/8/4P3/8/8/8/4K3 b - - 0 1").unwrap();
        game.make_move(Move {
            piece: black(PieceType::Pawn),
            from: Square::D7,
            to: Square::D5,
        });
        assert_eq!(game.game_state.en_passant, Some(Square::D6));
        let legal: HashSet<Square> =
            to_hash_set(game.get_legal_moves(white(PieceType::Pawn), Square::E5));
        assert!(
            legal.contains(&Square::D6),
            "e.p. should be legal right after the push"
        );
    }

    // endregion

    // region: make_move

    #[test]
    fn make_move_moves_the_piece_and_clears_origin() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E4,
        });
        assert!(ok);
        assert_eq!(game.game_state.get_piece(Square::E2), None);
        assert_eq!(
            game.game_state.get_piece(Square::E4),
            Some(white(PieceType::Pawn))
        );
    }

    #[test]
    fn make_move_toggles_side_to_move() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        assert_eq!(game.game_state.side, Side::White);
        game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E4,
        });
        assert_eq!(game.game_state.side, Side::Black);
    }

    #[test]
    fn make_move_rejects_illegal_move_and_does_not_mutate() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E5, // too far
        });
        assert!(!ok);
        assert_eq!(
            game.game_state.get_piece(Square::E2),
            Some(white(PieceType::Pawn))
        );
        assert_eq!(game.game_state.get_piece(Square::E5), None);
        assert_eq!(game.game_state.side, Side::White);
    }

    #[test]
    fn make_move_rejects_moving_opponent_piece() {
        let mut game = Game::new_game_from_fen("4k3/4p3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: black(PieceType::Pawn),
            from: Square::E7,
            to: Square::E5,
        });
        assert!(!ok);
        assert_eq!(game.game_state.side, Side::White);
    }

    #[test]
    fn make_move_captures_enemy_piece() {
        let mut game = Game::new_game_from_fen("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E4,
            to: Square::D5,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::D5),
            Some(white(PieceType::Pawn))
        );
    }

    #[test]
    fn make_move_white_double_push_sets_en_passant_behind_pawn() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E4,
        });
        assert_eq!(game.game_state.en_passant, Some(Square::E3));
    }

    #[test]
    fn make_move_black_double_push_sets_en_passant_behind_pawn() {
        let mut game = Game::new_game_from_fen("4k3/4p3/8/8/8/8/8/4K3 b - - 0 1").unwrap();
        game.make_move(Move {
            piece: black(PieceType::Pawn),
            from: Square::E7,
            to: Square::E5,
        });
        assert_eq!(game.game_state.en_passant, Some(Square::E6));
    }

    #[test]
    fn make_move_en_passant_removes_the_captured_pawn() {
        // White pawn e5, black pawn d5 just double-pushed (en passant target d6).
        let mut game = Game::new_game_from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E5,
            to: Square::D6,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::D6),
            Some(white(PieceType::Pawn))
        );
        assert_eq!(
            game.game_state.get_piece(Square::D5),
            None,
            "captured pawn must be removed"
        );
        assert_eq!(game.game_state.get_piece(Square::E5), None);
    }

    #[test]
    fn make_move_black_en_passant_removes_the_captured_pawn() {
        // Black pawn d4, white pawn e4 just double-pushed (en passant target e3).
        let mut game = Game::new_game_from_fen("4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: black(PieceType::Pawn),
            from: Square::D4,
            to: Square::E3,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::E4),
            None,
            "captured pawn must be removed"
        );
    }

    #[test]
    fn make_move_single_push_clears_en_passant() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w KQkq e6 0 1").unwrap();
        game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E3,
        });
        assert_eq!(game.game_state.en_passant, None);
    }

    #[test]
    fn make_move_king_move_revokes_both_castle_rights() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w KQ - 0 1").unwrap();
        game.make_move(Move {
            piece: white(PieceType::King),
            from: Square::E1,
            to: Square::E2,
        });
        assert!(!game.game_state.available_castle.white_kingside);
        assert!(!game.game_state.available_castle.white_queenside);
    }

    #[test]
    fn make_move_rook_move_revokes_that_side_castle_right() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
        game.make_move(Move {
            piece: white(PieceType::Rook),
            from: Square::H1,
            to: Square::H4,
        });
        assert!(!game.game_state.available_castle.white_kingside);
        assert!(game.game_state.available_castle.white_queenside);
    }

    #[test]
    fn make_move_kingside_castle_moves_rook() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::King),
            from: Square::E1,
            to: Square::G1,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::G1),
            Some(white(PieceType::King))
        );
        assert_eq!(
            game.game_state.get_piece(Square::F1),
            Some(white(PieceType::Rook))
        );
        assert_eq!(game.game_state.get_piece(Square::H1), None);
    }

    #[test]
    fn make_move_queenside_castle_moves_rook() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::King),
            from: Square::E1,
            to: Square::C1,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::C1),
            Some(white(PieceType::King))
        );
        assert_eq!(
            game.game_state.get_piece(Square::D1),
            Some(white(PieceType::Rook))
        );
        assert_eq!(game.game_state.get_piece(Square::A1), None);
    }

    #[test]
    fn make_move_black_kingside_castle_moves_rook() {
        let mut game = Game::new_game_from_fen("4k2r/8/8/8/8/8/8/4K3 b k - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: black(PieceType::King),
            from: Square::E8,
            to: Square::G8,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::F8),
            Some(black(PieceType::Rook))
        );
        assert_eq!(game.game_state.get_piece(Square::H8), None);
    }

    // endregion

    // region: pawn promotion

    #[test]
    fn white_pawn_promotes_to_queen_on_eighth_rank() {
        let mut game = Game::new_game_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: white(PieceType::Pawn),
            from: Square::A7,
            to: Square::A8,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::A8),
            Some(white(PieceType::Queen))
        );
        assert_eq!(game.game_state.get_piece(Square::A7), None);
    }

    #[test]
    fn black_pawn_promotes_to_queen_on_first_rank() {
        let mut game = Game::new_game_from_fen("4k3/8/8/8/8/8/p7/4K3 b - - 0 1").unwrap();
        let ok = game.make_move(Move {
            piece: black(PieceType::Pawn),
            from: Square::A2,
            to: Square::A1,
        });
        assert!(ok);
        assert_eq!(
            game.game_state.get_piece(Square::A1),
            Some(black(PieceType::Queen))
        );
    }

    #[test]
    fn pawn_promotes_to_chosen_piece() {
        let mut game = Game::new_game_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let mv = Move {
            piece: white(PieceType::Pawn),
            from: Square::A7,
            to: Square::A8,
        };
        assert!(game.make_move_promoting(mv, PieceType::Knight));
        assert_eq!(
            game.game_state.get_piece(Square::A8),
            Some(white(PieceType::Knight))
        );
    }

    #[test]
    fn cannot_promote_to_king_or_pawn() {
        let mut game = Game::new_game_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let mv = Move {
            piece: white(PieceType::Pawn),
            from: Square::A7,
            to: Square::A8,
        };
        assert!(!game.make_move_promoting(mv, PieceType::King));
        assert!(!game.make_move_promoting(mv, PieceType::Pawn));
    }

    #[test]
    fn promotion_without_choice_waits_for_the_player() {
        let mut game = Game::new_game_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        game.run_command("mv a7a8");
        assert!(game.pending_promotion.is_some());
        assert_eq!(
            game.game_state.get_piece(Square::A7),
            Some(white(PieceType::Pawn))
        );
        game.parse_input(&InputStatus::Promoting(Some(PieceType::Rook)));
        assert!(game.pending_promotion.is_none());
        assert_eq!(
            game.game_state.get_piece(Square::A8),
            Some(white(PieceType::Rook))
        );
    }

    #[test]
    fn terminal_command_promotes_to_chosen_piece() {
        let mut game = Game::new_game_from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        game.run_command("mv a7a8n");
        assert!(game.pending_promotion.is_none());
        assert_eq!(
            game.game_state.get_piece(Square::A8),
            Some(white(PieceType::Knight))
        );
    }

    // endregion

    // region: is_move_legal / check

    #[test]
    fn is_move_legal_rejects_move_leaving_king_in_check() {
        // Pawn e2 pinned by rook e8; stepping off the pin ray exposes the king.
        let game = Game::new_game_from_fen("4r3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        assert!(!game.is_move_legal(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::D3,
        }));
    }

    #[test]
    fn is_move_legal_allows_move_along_pin_ray() {
        let game = Game::new_game_from_fen("4r3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        assert!(game.is_move_legal(Move {
            piece: white(PieceType::Pawn),
            from: Square::E2,
            to: Square::E3,
        }));
    }

    #[test]
    fn is_move_legal_rejects_wrong_side() {
        let game = Game::new_game_from_fen("4k3/4p3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(!game.is_move_legal(Move {
            piece: black(PieceType::Pawn),
            from: Square::E7,
            to: Square::E6,
        }));
    }

    #[test]
    fn is_square_under_attack_true_for_attacked_square() {
        // Black rook on e8 attacks the whole e-file; side to move is White.
        let game = Game::new_game_from_fen("4r3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(game.is_square_under_attack(Square::E4));
    }

    #[test]
    fn is_square_under_attack_false_for_safe_square() {
        let game = Game::new_game_from_fen("4r3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(!game.is_square_under_attack(Square::A4));
    }

    // endregion
}

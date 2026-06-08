use crate::{
    game::{
        domain::{Board, Piece, PieceType, Side, Square},
        game_state::GameState,
    },
    inputs::handler::InputStatus,
};

pub struct Game {
    pub game_state: GameState,
}

impl Game {
    pub fn new() -> Self {
        Self {
            game_state: GameState::new(),
        }
    }

    pub fn new_game() -> Self {
        Self {
            game_state: GameState::new(),
        }
    }

    pub fn new_game_from_fen(fen: &str) -> Self {
        Self {
            game_state: Self::from_fen(fen),
        }
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        self.game_state.board[square as usize]
    }

    fn set_piece(&mut self, square: Square, piece: Piece) {
        self.game_state.board[square as usize] = Some(piece);
    }

    fn clear_square(&mut self, square: Square) -> bool {
        if self.get_piece(square).is_none() {
            return false;
        };

        self.game_state.board[square as usize] = None;
        true
    }

    fn move_piece(&mut self, from: Square, to: Square) -> bool {
        let Some(piece) = self.get_piece(from) else {
            return false;
        };

        // Check if legal move

        self.set_piece(to, piece);
        self.clear_square(from);
        true
    }

    pub fn parse_input(&mut self, input_status: &InputStatus) {
        match input_status {
            InputStatus::Chilling => { /* just chilling */ }
            InputStatus::Dragging(drag) => {
                println!(
                    "Moves: {:?}",
                    self.get_pseudo_legal_moves(drag.piece, drag.from)
                );
            }
            InputStatus::Releasing(drag, square) => {
                // controlla se square esiste o è uguale alla partenza
                if let Some(square) = *square {
                    if drag.from != square {
                        self.move_piece(drag.from, square);
                    }
                }
            }
        }
    }

    fn from_fen(fen: &str) -> GameState {
        let mut board: Board = [None; 64];

        let fen_parts: Vec<&str> = fen.split(' ').collect();
        let fen_board = fen_parts[0];

        let mut rank_index = 7;
        let mut file_index = 0;
        for c in fen_board.chars() {
            if c == '/' {
                file_index = 0;
                rank_index -= 1;
            } else if c.is_digit(10) {
                file_index += c.to_digit(10).unwrap() as usize;
            } else {
                let side = if c.is_uppercase() {
                    Side::White
                } else {
                    Side::Black
                };
                let piece_type = match c.to_ascii_lowercase() {
                    'k' => PieceType::King,
                    'q' => PieceType::Queen,
                    'r' => PieceType::Rook,
                    'b' => PieceType::Bishop,
                    'n' => PieceType::Knight,
                    'p' => PieceType::Pawn,
                    _ => panic!("Invalid piece type in FEN"),
                };
                let piece = Piece {
                    side,
                    kind: piece_type,
                };
                board[file_index + (rank_index * 8)] = Some(piece);
                file_index += 1;
            }
        }

        // TODO parse all the other parts
        let mut game_state = GameState::new();
        game_state.board = board;
        game_state
    }

    pub fn get_pseudo_legal_moves(&self, piece: Piece, square: Square) -> Vec<Square> {
        match piece.kind {
            PieceType::Pawn => self.get_pawn_pseudo_legal_moves(piece.side, square),
            PieceType::Knight => self.get_knight_pseudo_legal_moves(piece.side, square),
            PieceType::Bishop => todo!(),
            PieceType::Rook => todo!(),
            PieceType::Queen => todo!(),
            PieceType::King => todo!(),
        }
    }

    fn get_knight_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        let usquare = square as i8;
        let file = usquare % 8;
        let rank = usquare / 8;

        const DELTAS: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];

        let mut out = Vec::new();
        for (delta_file, delta_rank) in DELTAS {
            let candidate_file = file + delta_file;
            let candidate_rank = rank + delta_rank;
            if !(0..8).contains(&candidate_file) || !(0..8).contains(&candidate_rank) {
                continue;
            }
            let target = Square::from_index(candidate_rank * 8 + candidate_file).unwrap();
            match self.game_state.board[target as usize] {
                Some(p) if p.side == side => { /* block by side */ }
                _ => out.push(target),
            }
        }
        out
    }

    fn get_pawn_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        let usquare = square as i8;
        let file = usquare % 8;
        let rank = usquare / 8;

        let mut out = Vec::new();
        let sign: i8 = if side == Side::White { 1 } else { -1 };

        // Quite moves
        let one_step = (rank + sign) * 8 + file;

        if let Some(one_step) = Square::from_index(one_step) {
            if self.game_state.board[one_step as usize].is_none() {
                out.push(one_step);

                let on_start = matches!((side, rank), (Side::White, 1) | (Side::Black, 6));
                if on_start {
                    let two_step = (rank + 2 * sign) * 8 + file;
                    let two_sq = Square::from_index(two_step).unwrap();
                    if self.game_state.board[two_sq as usize].is_none() {
                        out.push(two_sq);
                    }
                }
            }
        }

        // Captures
        let candidate_idx_1 = (rank + 1 * sign) * 8 + file - 1;
        let candidate_idx_2 = (rank + 1 * sign) * 8 + file + 1;

        if let Some(candidate_square_1) = Square::from_index(candidate_idx_1) {
            if let Some(candidate_capture_1) = self.game_state.board[candidate_square_1 as usize] {
                if candidate_capture_1.side != side {
                    out.push(candidate_square_1);
                }
            }
        }

        if let Some(candidate_square_2) = Square::from_index(candidate_idx_2) {
            if let Some(candidate_capture_2) = self.game_state.board[candidate_square_2 as usize] {
                if candidate_capture_2.side != side {
                    out.push(candidate_square_2);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const EMPTY_FEN: &str = "8/8/8/8/8/8/8/8 w - - 0 1";

    fn moves_set(game: &Game, piece: Piece, sq: Square) -> HashSet<Square> {
        game.get_pseudo_legal_moves(piece, sq).into_iter().collect()
    }

    #[test]
    fn fen_starting_position_places_white_back_rank() {
        let game = Game::new_game_from_fen(START_FEN);
        assert_eq!(
            game.get_piece(Square::A1),
            Some(Piece {
                side: Side::White,
                kind: PieceType::Rook
            })
        );
        assert_eq!(
            game.get_piece(Square::E1),
            Some(Piece {
                side: Side::White,
                kind: PieceType::King
            })
        );
    }

    #[test]
    fn fen_starting_position_places_black_back_rank() {
        let game = Game::new_game_from_fen(START_FEN);
        assert_eq!(
            game.get_piece(Square::E8),
            Some(Piece {
                side: Side::Black,
                kind: PieceType::King
            })
        );
        assert_eq!(
            game.get_piece(Square::H8),
            Some(Piece {
                side: Side::Black,
                kind: PieceType::Rook
            })
        );
    }

    #[test]
    fn fen_starting_position_middle_ranks_are_empty() {
        let game = Game::new_game_from_fen(START_FEN);
        for sq_idx in 16..48 {
            let sq = Square::from_index(sq_idx).unwrap();
            assert!(game.get_piece(sq).is_none(), "expected {:?} empty", sq);
        }
    }

    #[test]
    fn knight_on_empty_board_has_eight_moves_from_center() {
        let game = Game::new_game_from_fen("8/8/8/8/3N4/8/8/8 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/4p3/5P2/3N4/8/8/8 w - - 0 1");
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
            EMPTY_FEN
                .replace("8/8/8/8/8/8/8/8", "8/8/8/8/8/8/4P3/8")
                .as_str(),
        );
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
        let game = Game::new_game_from_fen("8/8/8/8/8/4P3/8/8 w - - 0 1");
        let pawn = Piece {
            side: Side::White,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E3);
        assert_eq!(moves, [Square::E4].into_iter().collect());
    }

    #[test]
    fn white_pawn_blocked_cannot_advance() {
        let game = Game::new_game_from_fen("8/8/8/8/8/4p3/4P3/8 w - - 0 1");
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
    fn white_pawn_captures_diagonally() {
        let game = Game::new_game_from_fen("8/8/8/8/3p1p2/4P3/8/8 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/8/3p1P2/4P3/8/8 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/4p3/8/8/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/4p3/3PPp2/8/8/8/8/8 b - - 0 1");
        let pawn = Piece {
            side: Side::Black,
            kind: PieceType::Pawn,
        };
        let moves = moves_set(&game, pawn, Square::E7);
        let expected: HashSet<Square> = [Square::D6].into_iter().collect();
        assert_eq!(moves, expected);
    }
}

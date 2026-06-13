use std::io::Error;

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
            PieceType::Bishop => self.get_bishop_pseudo_legal_moves(piece.side, square),
            PieceType::Rook => self.get_rook_pseudo_legal_moves(piece.side, square),
            PieceType::Queen => self.get_queen_pseudo_legal_moves(piece.side, square),
            PieceType::King => self.get_king_pseudo_legal_moves(piece.side, square),
        }
    }

    fn piece_at(&self, square: Square) -> Option<Piece> {
        self.game_state.board[square as usize]
    }

    fn get_leaper_piece_pseudo_legal_moves(&self, piece: Piece, square: Square) -> Vec<Square> {
        const KNIGHT_DELTAS: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];
        const KING_DELTAS: [(i8, i8); 8] = [
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
        ];

        let directions: &[(i8, i8)] = match piece.kind {
            PieceType::Knight => &KNIGHT_DELTAS,
            PieceType::King => &KING_DELTAS,
            _ => panic!("Not a leaper piece"),
        };

        let mut moves = Vec::new();
        for (delta_file, delta_rank) in directions {
            let Some(target_square) =
                Square::from_file_rank(square.file() + delta_file, square.rank() + delta_rank)
            else {
                continue;
            };
            match self.piece_at(target_square) {
                Some(target_piece) if target_piece.side == piece.side => {}
                _ => moves.push(target_square),
            }
        }
        moves
    }

    fn get_knight_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        self.get_leaper_piece_pseudo_legal_moves(
            Piece {
                side,
                kind: PieceType::Knight,
            },
            square,
        )
    }

    fn get_king_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        self.get_leaper_piece_pseudo_legal_moves(
            Piece {
                side,
                kind: PieceType::King,
            },
            square,
        )
    }

    fn get_pawn_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        let file = square.file();
        let rank = square.rank();
        let direction = side.direction();

        let mut moves = Vec::new();

        // Quiet moves: single step, and double step from start rank.
        if let Some(one_step_square) = Square::from_file_rank(file, rank + direction) {
            if self.piece_at(one_step_square).is_none() {
                moves.push(one_step_square);
                if rank == side.pawn_start_rank() {
                    if let Some(two_step_square) =
                        Square::from_file_rank(file, rank + 2 * direction)
                    {
                        if self.piece_at(two_step_square).is_none() {
                            moves.push(two_step_square);
                        }
                    }
                }
            }
        }

        // Diagonal captures (skips off-board files, e.g. a/h-file edges).
        for capture_file_offset in [-1, 1] {
            let Some(capture_square) =
                Square::from_file_rank(file + capture_file_offset, rank + direction)
            else {
                continue;
            };
            if let Some(captured_piece) = self.piece_at(capture_square) {
                if captured_piece.side != side {
                    moves.push(capture_square);
                }
            }
        }

        moves
    }

    fn get_sliding_piece_legal_moves(&self, piece: Piece, square: Square) -> Vec<Square> {
        const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, -1), (-1, 1)];
        const ROOK_DIRECTIONS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        const QUEEN_DIRECTIONS: [(i8, i8); 8] = [
            (1, 1),
            (1, -1),
            (-1, -1),
            (-1, 1),
            (0, 1),
            (0, -1),
            (1, 0),
            (-1, 0),
        ];

        let directions: &[(i8, i8)] = match piece.kind {
            PieceType::Bishop => &BISHOP_DIRECTIONS,
            PieceType::Rook => &ROOK_DIRECTIONS,
            PieceType::Queen => &QUEEN_DIRECTIONS,
            _ => panic!("Not a sliding piece"),
        };

        let file = square.file();
        let rank = square.rank();

        let mut moves = Vec::new();

        for (direction_file, direction_rank) in directions {
            for square_inc in 1..8 {
                let Some(target_square) = Square::from_file_rank(
                    file + (direction_file * square_inc),
                    rank + (direction_rank * square_inc),
                ) else {
                    break;
                };

                match self.piece_at(target_square) {
                    Some(p) if p.side == piece.side => break,
                    Some(_) => {
                        moves.push(target_square);
                        break;
                    }
                    None => moves.push(target_square),
                }
            }
        }
        moves
    }

    fn get_bishop_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(
            Piece {
                side,
                kind: PieceType::Bishop,
            },
            square,
        )
    }

    fn get_rook_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(
            Piece {
                side,
                kind: PieceType::Rook,
            },
            square,
        )
    }

    fn get_queen_pseudo_legal_moves(&self, side: Side, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(
            Piece {
                side,
                kind: PieceType::Queen,
            },
            square,
        )
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
    fn a_file_pawn_does_not_wrap_to_h_file_on_capture() {
        // White pawn on a2; a black piece sits on h2 (would be "left capture" if file wraps).
        let game = Game::new_game_from_fen("8/8/8/8/8/8/P6p/8 w - - 0 1");
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

    #[test]
    fn bishop_can_move_in_all_squares_not_blocked_by_friendly_pieces() {
        let game = Game::new_game_from_fen("8/8/2N5/8/4B3/8/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/2n5/8/4B3/8/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/3b4/4k3/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4b3/4r3/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("7p/1K6/1N6/4q3/4R3/4r3/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/N7 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/Nk6 w - - 0 1");
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
}

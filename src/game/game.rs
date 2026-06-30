use crate::{
    game::{
        domain::{Board, Move, Piece, PieceType, Side, Square},
        game_state::{CastleRights, GameState},
    },
    inputs::handler::InputStatus,
};

pub struct Game {
    pub game_state: GameState,
}

impl Game {
    // region: game
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

    // endregion

    // region: Inputs
    pub fn parse_input(&mut self, input_status: &InputStatus) {
        match input_status {
            InputStatus::Chilling => { /* just chilling */ }
            InputStatus::Dragging(drag) => {
                // Draw dots for legal moves
                println!("Moves: {:?}", self.get_legal_moves(drag.piece, drag.from));
                println!("Side -> {}", self.is_square_under_attack(drag.from));
            }
            InputStatus::Releasing(drag, square) => {
                if let Some(square) = *square {
                    if drag.from != square {
                        self.game_state.move_piece(drag.from, square); // TODO use make_move
                    }
                }
            }
        }
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        self.game_state.get_piece(square)
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

        let mut game_state = GameState::new();
        game_state.board = board;

        game_state.side = match fen_parts[1].to_ascii_lowercase().as_str() {
            "b" => Side::Black,
            "w" => Side::White,
            _ => panic!("Invalid side type in FEN"),
        };

        let castle_field = fen_parts[2];
        game_state.available_castle = CastleRights {
            white_kingside: castle_field.contains('K'),
            white_queenside: castle_field.contains('Q'),
            black_kingside: castle_field.contains('k'),
            black_queenside: castle_field.contains('q'),
        };

        let en_passant = fen_parts[3];
        game_state.en_passant = match en_passant {
            "-" => None,
            s => Some(s.parse().expect("invalid en passant square in FEN")),
        };

        // TODO parse all the other parts and validate it overall

        game_state
    }

    // endregion

    // region: moves

    fn make_move(&mut self, mv: Move) -> bool {
        if self.game_state.move_piece(mv.from, mv.to) {
            // Check if legal moves

            // Check enpassant

            // Check castle
            true
        } else {
            false
        }
    }

    fn is_move_legal(&self, mv: Move) -> bool {
        let mut game_state_snapshot = self.game_state.clone();

        if game_state_snapshot.move_piece(mv.from, mv.to) {
            for i in 0..64u8 {
                let square: Square = unsafe { std::mem::transmute(i) };
                if game_state_snapshot.get_piece(square).is_some_and(|p| {
                    p.kind == PieceType::King && p.side == game_state_snapshot.side
                }) && self._is_square_under_attack(game_state_snapshot, square)
                {
                    return false;
                }
            }
        } else {
            return false;
        }
        return true;
    }

    fn get_squares_under_attacks(&self) -> Vec<Square> {
        let mut attacked_squares = Vec::new();

        for i in 0..64u8 {
            let square: Square = unsafe { std::mem::transmute(i) };
            if self.is_square_under_attack(square) {
                attacked_squares.push(square);
            }
        }
        attacked_squares
    }

    fn is_square_under_attack(&self, square: Square) -> bool {
        self._is_square_under_attack(self.game_state, square)
    }

    fn _is_square_under_attack(&self, state: GameState, square: Square) -> bool {
        let file = square.file();
        let rank = square.rank();
        let attacker = if state.side == Side::White {
            Side::Black
        } else {
            Side::White
        };
        let direction = state.side.direction();

        // By pawns
        for pawn_attacking_deltas in [-1, 1] {
            let Some(pawn_attacking_square_candidate) =
                Square::from_file_rank(file + pawn_attacking_deltas, rank + direction)
            else {
                continue;
            };
            if let Some(pawn_attacker_piece_candidate) =
                state.piece_at(pawn_attacking_square_candidate)
            {
                if pawn_attacker_piece_candidate.kind == PieceType::Pawn
                    && pawn_attacker_piece_candidate.side == attacker
                {
                    return true;
                }
            }
        }

        // By Knight
        let knight_deltas: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];

        for (delta_file, delta_rank) in knight_deltas {
            let Some(target_square) =
                Square::from_file_rank(square.file() + delta_file, square.rank() + delta_rank)
            else {
                continue;
            };
            if let Some(knight_attacker_piece_candidate) = state.piece_at(target_square) {
                if knight_attacker_piece_candidate.kind == PieceType::Knight
                    && knight_attacker_piece_candidate.side == attacker
                {
                    return true;
                }
            }
        }

        // By King
        let king_deltas: [(i8, i8); 8] = [
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
        ];
        for (delta_file, delta_rank) in king_deltas {
            let Some(target_square) =
                Square::from_file_rank(square.file() + delta_file, square.rank() + delta_rank)
            else {
                continue;
            };
            if let Some(king_attacker_piece_candidate) = state.piece_at(target_square) {
                if king_attacker_piece_candidate.kind == PieceType::King
                    && king_attacker_piece_candidate.side == attacker
                {
                    return true;
                }
            }
        }

        // By bishop (or Queen)
        let bishop_directions: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, -1), (-1, 1)];

        for (direction_file, direction_rank) in bishop_directions {
            for square_inc in 1..8 {
                let Some(target_square) = Square::from_file_rank(
                    square.file() + (direction_file * square_inc),
                    square.rank() + (direction_rank * square_inc),
                ) else {
                    break;
                };

                if let Some(piece) = state.piece_at(target_square) {
                    if (piece.kind == PieceType::Bishop || piece.kind == PieceType::Queen)
                        && piece.side == attacker
                    {
                        return true;
                    }

                    break;
                }
            }
        }

        // By Rook (or Queen)
        let rook_directions: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (direction_file, direction_rank) in rook_directions {
            for square_inc in 1..8 {
                let Some(target_square) = Square::from_file_rank(
                    square.file() + (direction_file * square_inc),
                    square.rank() + (direction_rank * square_inc),
                ) else {
                    break;
                };

                if let Some(piece) = state.piece_at(target_square) {
                    if (piece.kind == PieceType::Rook || piece.kind == PieceType::Queen)
                        && piece.side == attacker
                    {
                        return true;
                    }

                    break;
                }
            }
        }

        false
    }

    pub fn get_legal_moves(&self, piece: Piece, square: Square) -> Vec<Square> {
        self.get_pseudo_legal_moves(piece, square)
            .iter()
            .filter(|&&to| {
                self.is_move_legal(Move {
                    piece,
                    from: square,
                    to: to,
                })
            })
            .copied()
            .collect()
    }

    fn get_pseudo_legal_moves(&self, piece: Piece, square: Square) -> Vec<Square> {
        match piece.kind {
            PieceType::Pawn => self.get_pawn_pseudo_legal_moves(square),
            PieceType::Knight => self.get_knight_pseudo_legal_moves(square),
            PieceType::Bishop => self.get_bishop_pseudo_legal_moves(square),
            PieceType::Rook => self.get_rook_pseudo_legal_moves(square),
            PieceType::Queen => self.get_queen_pseudo_legal_moves(square),
            PieceType::King => self.get_king_pseudo_legal_moves(square),
        }
    }

    fn get_leaper_piece_pseudo_legal_moves(&self, kind: PieceType, square: Square) -> Vec<Square> {
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

        let directions: &[(i8, i8)] = match kind {
            PieceType::Knight => &KNIGHT_DELTAS,
            PieceType::King => &KING_DELTAS,
            _ => panic!("Not a leaper piece"),
        };

        let side = self.game_state.side;
        let mut moves = Vec::new();
        for (delta_file, delta_rank) in directions {
            let Some(target_square) =
                Square::from_file_rank(square.file() + delta_file, square.rank() + delta_rank)
            else {
                continue;
            };
            match self.game_state.piece_at(target_square) {
                Some(target_piece) if target_piece.side == side => {}
                _ => moves.push(target_square),
            }
        }
        moves
    }

    fn get_knight_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        self.get_leaper_piece_pseudo_legal_moves(PieceType::Knight, square)
    }

    fn get_king_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        let side = self.game_state.side;
        let mut leaper_moves = self.get_leaper_piece_pseudo_legal_moves(PieceType::King, square);

        // Castle
        let rights = self.game_state.available_castle;
        let (kingside, queenside) = match side {
            Side::White => (rights.white_kingside, rights.white_queenside),
            Side::Black => (rights.black_kingside, rights.black_queenside),
        };

        if !self.is_square_under_attack(square) {
            if kingside {
                match side {
                    Side::White => {
                        if self.game_state.piece_at(Square::F1).is_none()
                            && self.game_state.piece_at(Square::G1).is_none()
                            && !self.is_square_under_attack(Square::F1)
                            && !self.is_square_under_attack(Square::G1)
                        {
                            leaper_moves.push(Square::G1)
                        }
                    }
                    Side::Black => {
                        if self.game_state.piece_at(Square::F8).is_none()
                            && self.game_state.piece_at(Square::G8).is_none()
                            && !self.is_square_under_attack(Square::F8)
                            && !self.is_square_under_attack(Square::G8)
                        {
                            leaper_moves.push(Square::G8)
                        }
                    }
                };
            }

            if queenside {
                match side {
                    Side::White => {
                        if self.game_state.piece_at(Square::B1).is_none()
                            && self.game_state.piece_at(Square::C1).is_none()
                            && self.game_state.piece_at(Square::D1).is_none()
                            && !self.is_square_under_attack(Square::D1)
                            && !self.is_square_under_attack(Square::C1)
                        {
                            leaper_moves.push(Square::C1)
                        }
                    }
                    Side::Black => {
                        if self.game_state.piece_at(Square::B8).is_none()
                            && self.game_state.piece_at(Square::C8).is_none()
                            && self.game_state.piece_at(Square::D8).is_none()
                            && !self.is_square_under_attack(Square::D8)
                            && !self.is_square_under_attack(Square::C8)
                        {
                            leaper_moves.push(Square::C8)
                        }
                    }
                };
            }
        }

        leaper_moves // TODO
    }

    fn get_pawn_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        let side = self.game_state.side;
        let file = square.file();
        let rank = square.rank();
        let direction = side.direction();

        let mut moves = Vec::new();

        // Quiet moves: single step, and double step from start rank.
        if let Some(one_step_square) = Square::from_file_rank(file, rank + direction) {
            if self.game_state.piece_at(one_step_square).is_none() {
                moves.push(one_step_square);
                if rank == side.pawn_start_rank() {
                    if let Some(two_step_square) =
                        Square::from_file_rank(file, rank + 2 * direction)
                    {
                        if self.game_state.piece_at(two_step_square).is_none() {
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
            if let Some(captured_piece) = self.game_state.piece_at(capture_square) {
                if captured_piece.side != side {
                    moves.push(capture_square);
                }
            }
        }

        // En passant

        if let Some(en_passant_square) = self.game_state.en_passant.clone() {
            if Square::from_file_rank(file + 1, rank + direction)
                .is_some_and(|sq| sq == en_passant_square)
            {
                moves.push(en_passant_square);
            }
            if Square::from_file_rank(file - 1, rank + direction)
                .is_some_and(|sq| sq == en_passant_square)
            {
                moves.push(en_passant_square);
            }
        }

        moves
    }

    fn get_sliding_piece_legal_moves(&self, kind: PieceType, square: Square) -> Vec<Square> {
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

        let directions: &[(i8, i8)] = match kind {
            PieceType::Bishop => &BISHOP_DIRECTIONS,
            PieceType::Rook => &ROOK_DIRECTIONS,
            PieceType::Queen => &QUEEN_DIRECTIONS,
            _ => panic!("Not a sliding piece"),
        };

        let side = self.game_state.side;
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

                match self.game_state.piece_at(target_square) {
                    Some(p) if p.side == side => break,
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

    fn get_bishop_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(PieceType::Bishop, square)
    }

    fn get_rook_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(PieceType::Rook, square)
    }

    fn get_queen_pseudo_legal_moves(&self, square: Square) -> Vec<Square> {
        self.get_sliding_piece_legal_moves(PieceType::Queen, square)
    }

    // endregion
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
        let game = Game::new_game_from_fen(START_FEN);
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
        let game = Game::new_game_from_fen(START_FEN);
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/3b4/4k3/8/N7 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4b3/4r3/8/N7 b - - 0 1");
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
        let game = Game::new_game_from_fen("7p/1K6/1N6/4q3/4R3/4r3/8/N7 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/1K6/1N6/4q3/4R3/4r3/8/Nk6 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/8/3P4/8/8/8 b - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::C5, Square::E5]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_pawns() {
        let game = Game::new_game_from_fen("8/1P6/8/8/8/8/6P1/8 b - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::A8, Square::C8, Square::F3, Square::H3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_white_pawns() {
        let game = Game::new_game_from_fen("8/1P6/8/8/8/8/6P1/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_pawn() {
        let game = Game::new_game_from_fen("8/8/8/8/3p4/8/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::C3, Square::E3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_pawns() {
        let game = Game::new_game_from_fen("8/7p/8/8/8/1p6/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::G6, Square::A2, Square::C2]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_black_pawns() {
        let game = Game::new_game_from_fen("8/7p/8/8/8/1p6/8/8 b - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_knight() {
        let game = Game::new_game_from_fen("8/8/8/4N3/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/4N3/8/4N3/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/1N6/8/8/6N1/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_knight() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/7n w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::F2, Square::G3]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn no_squares_are_under_attack_by_black_knight() {
        let game = Game::new_game_from_fen("8/8/8/8/8/8/8/7n b - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_king() {
        let game = Game::new_game_from_fen("8/8/8/4K3/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/4K3/8/8/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_black_king() {
        let game = Game::new_game_from_fen("k7/8/8/4K3/8/8/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([Square::A7, Square::B7, Square::B8]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_bishop() {
        let game = Game::new_game_from_fen("8/8/8/3B4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/5P2/8/3B4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/5p2/8/3B4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/3b4/8/8/8/8 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/3B4/8/8/8/8 w - - 0 1");
        let squares = to_hash_set(game.get_squares_under_attacks());
        let expected = to_hash_set([]);

        assert_eq!(squares, expected);
    }

    #[test]
    fn squares_are_under_attack_by_white_rook() {
        let game = Game::new_game_from_fen("8/8/8/3R4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/3P4/8/3R4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("8/3p4/8/3R4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("r7/8/8/8/8/8/8/8 w - - 0 1");
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
        let game = Game::new_game_from_fen("8/8/8/3Q4/8/8/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_without_rights() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K2R w - - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_path_occupied() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4KB1R w K - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_transit_attacked() {
        let game = Game::new_game_from_fen("4kr2/8/8/8/8/8/8/4K2R w K - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_kingside_when_destination_attacked() {
        let game = Game::new_game_from_fen("4k1r1/8/8/8/8/8/8/4K2R w K - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_cannot_castle_when_in_check() {
        let game = Game::new_game_from_fen("4k3/4r3/8/8/8/8/8/4K2R w K - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::G1));
    }

    #[test]
    fn white_king_can_castle_queenside_when_path_clear() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(moves.contains(&Square::C1));
    }

    #[test]
    fn white_king_cannot_castle_queenside_when_b1_occupied() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/RN2K3 w Q - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::C1));
    }

    #[test]
    fn white_king_cannot_castle_queenside_when_d1_attacked() {
        let game = Game::new_game_from_fen("3rk3/8/8/8/8/8/8/R3K3 w Q - 0 1");
        let king = Piece {
            side: Side::White,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E1);
        assert!(!moves.contains(&Square::C1));
    }

    #[test]
    fn black_king_can_castle_both_sides() {
        let game = Game::new_game_from_fen("r3k2r/8/8/8/8/8/8/4K3 b kq - 0 1");
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
        let game = Game::new_game_from_fen("4k2r/8/8/8/8/B7/8/4K3 b k - 0 1");
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let moves = moves_set(&game, king, Square::E8);
        assert!(!moves.contains(&Square::G8));
    }

    #[test]
    fn white_queen_blocked_on_all_rays() {
        let game = Game::new_game_from_fen("8/8/8/2PPP3/2PQP3/2PPP3/8/8 b - - 0 1");
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
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1");
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
        let game = Game::new_game_from_fen("k3r3/8/8/8/6b1/8/4P3/3K4 w - - 0 1");
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
        );
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
        );
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
        );
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
            Game::new_game_from_fen("r3k3/pppnqNpp/8/4p3/Q1Bn2b1/2P5/PP1P1KPP/RNB4R b q - 0 13");
        let king = Piece {
            side: Side::Black,
            kind: PieceType::King,
        };
        let legal: HashSet<Square> = to_hash_set(game.get_legal_moves(king, Square::E8));
        let expected: HashSet<Square> = to_hash_set([Square::F8]);
        assert_eq!(legal, expected);
    }
}

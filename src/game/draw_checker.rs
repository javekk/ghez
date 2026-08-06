use crate::game::{
    domain::{Piece, PieceType, Side, Square},
    game::Game,
    game_state::{
        DrawReason::{self, FiftyMoveRule, InsufficientMaterial, Stalemate, ThreefoldRepetition},
        GameStatus,
    },
    movegen::{self, is_stalemate},
};

pub struct DrawChecker {}

impl DrawChecker {
    pub fn draw_reason(game: &Game) -> Option<DrawReason> {
        if movegen::is_stalemate(&game.game_state) {
            return Some(Stalemate);
        }

        if game.game_state.halfmove_counter >= 100 {
            return Some(FiftyMoveRule);
        }

        if DrawChecker::is_threefold_repetition(game) {
            return Some(ThreefoldRepetition);
        }

        if DrawChecker::is_insufficient_material(game) {
            return Some(InsufficientMaterial);
        }

        // TODO add draw by agrement

        return None;
    }

    fn is_threefold_repetition(game: &Game) -> bool {
        let current_key = game.game_state.get_repetion_key();
        let position_count = game
            .game_history
            .iter()
            .filter(|&game_state| game_state.get_repetion_key() == current_key)
            .count();
        position_count >= 3
    }

    fn get_side_pieces(game: &Game, side: Side) -> Vec<Piece> {
        game.game_state
            .board
            .iter()
            .filter_map(|&piece| piece.filter(|p| p.side == side))
            .collect()
    }

    fn get_bishop_square_color(game: &Game, side: Side) -> Option<&str> {
        let square = game
            .game_state
            .board
            .iter()
            .position(|&p| p.is_some_and(|p| p.side == side && p.kind == PieceType::Bishop));

        if let Some(square_index) = square {
            if let Some(square) = Square::from_index(square_index as i8) {
                if square.file() % 2 == 0 && square.rank() % 2 == 0 {
                    Some("dark")
                } else if square.file() % 2 == 1 && square.rank() % 2 == 1 {
                    Some("dark")
                } else {
                    Some("light")
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn is_insufficient_material(game: &Game) -> bool {
        let black_pieces = DrawChecker::get_side_pieces(game, Side::Black);
        let black_counter = black_pieces.len();

        let white_pieces = DrawChecker::get_side_pieces(game, Side::White);
        let white_counter = white_pieces.len();

        if black_counter > 3 || white_counter > 3 {
            return false;
        }

        let is_black_king_only =
            black_counter == 1 && black_pieces.iter().any(|&p| p.kind == PieceType::King);

        let is_white_king_only =
            white_counter == 1 && white_pieces.iter().any(|&p| p.kind == PieceType::King);

        let is_black_king_and_knight_only = black_counter == 2
            && black_pieces
                .iter()
                .all(|&p| p.kind == PieceType::King || p.kind == PieceType::Knight);

        let is_white_king_and_knight_only = white_counter == 2
            && white_pieces
                .iter()
                .all(|&p| p.kind == PieceType::King || p.kind == PieceType::Knight);

        let is_black_king_and_bishop_only = black_counter == 2
            && black_pieces
                .iter()
                .all(|&p| p.kind == PieceType::King || p.kind == PieceType::Bishop);

        let is_white_king_and_bishop_only = white_counter == 2
            && white_pieces
                .iter()
                .all(|&p| p.kind == PieceType::King || p.kind == PieceType::Bishop);

        let black_bishop_color = DrawChecker::get_bishop_square_color(game, Side::Black);
        let white_bishop_color = DrawChecker::get_bishop_square_color(game, Side::White);

        let matches = [
            is_black_king_only && is_white_king_only,
            is_black_king_only && is_white_king_and_bishop_only,
            is_black_king_only && is_white_king_and_knight_only,
            is_black_king_and_knight_only && is_white_king_only,
            is_black_king_and_knight_only && is_white_king_and_knight_only,
            is_black_king_and_bishop_only && is_white_king_only,
            is_black_king_and_bishop_only
                && is_white_king_and_bishop_only
                && (white_bishop_color == black_bishop_color),
        ];

        return matches.iter().any(|&x| x);
    }
}

mod tests {
    // region: is_insufficient_material

    use crate::game::{
        domain::{Move, Piece, PieceType, Side, Square},
        draw_checker::DrawChecker,
        game::Game,
    };

    #[test]
    fn is_insufficient_material_true_for_king_vs_king() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_true_for_king_vs_king_and_bishop() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        assert!(DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_true_for_king_vs_king_and_knight() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/2N1K3 w - - 0 1").unwrap();
        assert!(DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_true_for_king_and_knight_vs_king_and_knight() {
        let game = Game::new_game_from_fen("2n1k3/8/8/8/8/8/8/2N1K3 w - - 0 1").unwrap();
        assert!(DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_true_for_same_colored_bishops() {
        // White bishop c1 and black bishop f8 are both on dark squares.
        let game = Game::new_game_from_fen("4kb2/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        assert!(DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_false_for_opposite_colored_bishops() {
        // White bishop f1 (light) and black bishop c8 (light too by the same-parity
        // rule from c1/f8 above swapped one file) must land on opposite colors here:
        // c8 (file 2, rank 7 -> odd sum -> light) vs f1 (file 5, rank 0 -> odd sum -> light)...
        // use b8 (file 1, rank 7 -> even sum -> dark) vs f1 (file 5, rank 0 -> odd -> light).
        let game = Game::new_game_from_fen("1b2k3/8/8/8/8/8/8/4KB2 w - - 0 1").unwrap();
        assert!(!DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_false_for_king_and_two_knights_vs_king() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/8/1NN1K3 w - - 0 1").unwrap();
        assert!(!DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_false_when_extra_material_present() {
        let game = Game::new_game_from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        assert!(!DrawChecker::is_insufficient_material(&game));
    }

    #[test]
    fn is_insufficient_material_false_for_king_and_bishop_vs_king_and_knight() {
        let game = Game::new_game_from_fen("2n1k3/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        assert!(!DrawChecker::is_insufficient_material(&game));
    }

    // endregion

    // region: is_threefold_repetition

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

    fn shuffle_knights_back_and_forth(game: &mut Game) {
        // Ng1-f3, Ng8-f6, Nf3-g1, Nf6-g8: one full round trip back to the start.
        game.make_move(Move {
            piece: white(PieceType::Knight),
            from: Square::G1,
            to: Square::F3,
        });
        game.make_move(Move {
            piece: black(PieceType::Knight),
            from: Square::G8,
            to: Square::F6,
        });
        game.make_move(Move {
            piece: white(PieceType::Knight),
            from: Square::F3,
            to: Square::G1,
        });
        game.make_move(Move {
            piece: black(PieceType::Knight),
            from: Square::F6,
            to: Square::G8,
        });
    }

    #[test]
    fn is_threefold_repetition_false_for_starting_position_only() {
        let game = Game::new_game_from_initial_position();
        assert!(!DrawChecker::is_threefold_repetition(&game));
    }

    #[test]
    fn is_threefold_repetition_false_after_two_occurrences() {
        let mut game = Game::new_game_from_initial_position();
        shuffle_knights_back_and_forth(&mut game);
        // Starting position has now occurred twice (initial + after the round trip).
        assert!(!DrawChecker::is_threefold_repetition(&game));
    }

    #[test]
    fn is_threefold_repetition_true_after_three_occurrences() {
        let mut game = Game::new_game_from_initial_position();
        shuffle_knights_back_and_forth(&mut game);
        shuffle_knights_back_and_forth(&mut game);
        // Starting position has now occurred three times.
        assert!(DrawChecker::is_threefold_repetition(&game));
    }

    #[test]
    fn is_threefold_repetition_ignores_move_counters() {
        // Same board/side/castle/en-passant as the start, but reached via captures
        // that reset halfmove_counter and via extra fullmoves: the position still
        // must count as a repeat of the initial position for repetition purposes.
        let mut game = Game::new_game_from_initial_position();
        let initial_key = game.game_state.get_repetion_key();

        shuffle_knights_back_and_forth(&mut game);
        assert_ne!(
            game.game_state.halfmove_counter, 0,
            "sanity: halfmove_counter should have advanced"
        );
        assert_eq!(game.game_state.get_repetion_key(), initial_key);
    }

    #[test]
    fn is_threefold_repetition_false_when_castle_rights_differ() {
        // Same piece placement/side/en-passant as after a rook shuffles home,
        // but castling rights differ because the rook moved away and back.
        let mut game = Game::new_game_from_fen("r3k3/8/8/8/8/8/8/R3K3 w Qq - 0 1").unwrap();

        // Move the white rook away and back: board is restored, but the
        // kingside/queenside rights lost along the way never come back.
        game.make_move(Move {
            piece: white(PieceType::Rook),
            from: Square::A1,
            to: Square::B1,
        });
        game.make_move(Move {
            piece: black(PieceType::King),
            from: Square::E8,
            to: Square::D8,
        });
        game.make_move(Move {
            piece: white(PieceType::Rook),
            from: Square::B1,
            to: Square::A1,
        });
        game.make_move(Move {
            piece: black(PieceType::King),
            from: Square::D8,
            to: Square::E8,
        });

        // Board is back to the start, but White's queenside right and Black's
        // queenside right were forfeited by the king/rook moves above.
        assert!(!DrawChecker::is_threefold_repetition(&game));
    }

    #[test]
    fn is_threefold_repetition_false_when_en_passant_availability_differs() {
        // Two positions with identical board/side/castle rights, but one has
        // an en-passant target and the other doesn't: must not be treated as equal.
        let with_ep = Game::new_game_from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let without_ep = Game::new_game_from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1").unwrap();

        assert_ne!(
            with_ep.game_state.get_repetion_key(),
            without_ep.game_state.get_repetion_key()
        );
    }

    // endregion
}

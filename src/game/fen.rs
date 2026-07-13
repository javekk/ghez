use crate::game;
use crate::game::domain::{Board, Piece, PieceType, Side, Square};
use crate::game::game_state::{self, CastleRights, GameState};

/// Parse a FEN string into a [`GameState`].
///
/// Only the piece placement, side to move, castling rights, and en passant
/// target are read;
pub fn parse(fen: &str) -> Result<GameState, String> {
    let fields: Vec<&str> = fen.split(' ').collect();
    let [board, side, castle, en_passant, ..] = fields[..] else {
        return Err("FEN must have at least 4 fields".to_owned());
    };

    let mut state = GameState::new();
    state.board = parse_board(board).map_err(|e| format!("Unparsable FEN (board): {e}"))?;
    state.side = parse_side(side).map_err(|_| "Unparsable FEN (side)")?;
    state.available_castle = parse_castle_rights(castle);
    state.en_passant = parse_en_passant(en_passant).map_err(|_| "Unparsable FEN (en passant)")?;
    state.halfmove_counter = fields
        .get(4)
        .map_or(Ok(0), |f| f.parse())
        .map_err(|_| "Unparsable FEN (halfmove counter)")?;
    state.fullmove_number = fields
        .get(5)
        .map_or(Ok(1), |f| f.parse())
        .map_err(|_| "Unparsable FEN (en passant)")?;
    Ok(state)
}

fn parse_board(placement: &str) -> Result<Board, String> {
    let mut board: Board = [None; 64];
    let mut rank = 7;
    let mut file = 0;

    for symbol in placement.chars() {
        match symbol {
            '/' => {
                rank -= 1;
                file = 0;
            }
            digit if digit.is_ascii_digit() => {
                file += digit.to_digit(10).unwrap() as usize;
            }
            piece => {
                board[rank * 8 + file] = Some(parse_piece(piece)?);
                file += 1;
            }
        }
    }

    Ok(board)
}

fn parse_piece(symbol: char) -> Result<Piece, String> {
    let side = if symbol.is_uppercase() {
        Side::White
    } else {
        Side::Black
    };

    let kind = match symbol.to_ascii_lowercase() {
        'k' => PieceType::King,
        'q' => PieceType::Queen,
        'r' => PieceType::Rook,
        'b' => PieceType::Bishop,
        'n' => PieceType::Knight,
        'p' => PieceType::Pawn,
        _ => return Err(format!("invalid piece '{symbol}'")),
    };

    Ok(Piece { side, kind })
}

fn parse_side(field: &str) -> Result<Side, String> {
    match field {
        "w" => Ok(Side::White),
        "b" => Ok(Side::Black),
        _ => Err("invalid side '{field}' in FEN".to_owned()),
    }
}

fn parse_castle_rights(field: &str) -> CastleRights {
    CastleRights {
        white_kingside: field.contains('K'),
        white_queenside: field.contains('Q'),
        black_kingside: field.contains('k'),
        black_queenside: field.contains('q'),
    }
}

fn parse_en_passant(field: &str) -> Result<Option<Square>, String> {
    match field {
        "-" => Ok(None),
        square => square
            .parse()
            .map(Some)
            .map_err(|_| format!("bad en passant square: {square}")),
    }
}

pub fn to_fen(game_state: GameState) -> String {
    let mut fen = String::new();

    // Board
    for rank_index in (0..8).rev() {
        let mut inc = 0;
        for file_index in 0..8 {
            if let Some(square) = Square::from_file_rank(file_index, rank_index) {
                if let Some(piece) = game_state.get_piece(square) {
                    if inc > 0 {
                        fen.push_str(&(inc).to_string());
                        inc = 0;
                    }
                    fen.push_str(piece.to_string());
                } else {
                    inc = inc + 1;
                }
            }
        }
        if inc > 0 {
            fen.push_str(&(inc).to_string());
        }
        if rank_index > 0 {
            fen.push_str("/");
        }
    }

    fen.push_str(" ");

    // Side
    fen.push_str(if game_state.side == Side::Black {
        "b"
    } else {
        "w"
    });

    fen.push_str(" ");

    // Castle rights
    if game_state.available_castle.white_kingside {
        fen.push_str("K");
    }
    if game_state.available_castle.white_queenside {
        fen.push_str("Q");
    }
    if game_state.available_castle.black_kingside {
        fen.push_str("k");
    }
    if game_state.available_castle.black_queenside {
        fen.push_str("q");
    }
    if !game_state.available_castle.is_castle_still_available() {
        fen.push_str("-");
    }

    fen.push_str(" ");

    // En passant
    if let Some(square) = game_state.en_passant {
        fen.push_str(&square.to_string())
    } else {
        fen.push_str("-");
    }

    fen.push_str(" ");

    // halfmove clock
    fen.push_str(&game_state.halfmove_counter.to_string());

    fen.push_str(" ");

    // full move number
    fen.push_str(&game_state.fullmove_number.to_string());
    fen
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn piece(side: Side, kind: PieceType) -> Option<Piece> {
        Some(Piece { side, kind })
    }

    #[test]
    fn parses_starting_position_pieces() {
        let state = parse(START).unwrap();
        assert_eq!(
            state.board[Square::A1 as usize],
            piece(Side::White, PieceType::Rook)
        );
        assert_eq!(
            state.board[Square::E1 as usize],
            piece(Side::White, PieceType::King)
        );
        assert_eq!(
            state.board[Square::E8 as usize],
            piece(Side::Black, PieceType::King)
        );
        assert_eq!(
            state.board[Square::B8 as usize],
            piece(Side::Black, PieceType::Knight)
        );
        assert_eq!(state.board[Square::E4 as usize], None);
        assert_eq!(state.board.iter().filter(|s| s.is_some()).count(), 32);
    }

    #[test]
    fn parses_starting_position_metadata() {
        let state = parse(START).unwrap();
        assert_eq!(state.side, Side::White);
        assert!(state.available_castle.white_kingside);
        assert!(state.available_castle.black_queenside);
        assert_eq!(state.en_passant, None);
        assert_eq!(state.halfmove_counter, 0);
        assert_eq!(state.fullmove_number, 1);
    }

    #[test]
    fn parses_black_to_move() {
        let state = parse("8/8/8/8/8/5k2/4p3/4K3 b - - 0 1").unwrap();
        assert_eq!(state.side, Side::Black);
    }

    #[test]
    fn parses_partial_castle_rights() {
        let state = parse("r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1").unwrap();
        assert!(state.available_castle.white_kingside);
        assert!(!state.available_castle.white_queenside);
        assert!(!state.available_castle.black_kingside);
        assert!(state.available_castle.black_queenside);
    }

    #[test]
    fn parses_en_passant_square() {
        let state = parse("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
        assert_eq!(state.en_passant, Some(Square::E3));
    }

    #[test]
    fn parses_move_counters() {
        let state = parse("8/8/8/8/8/5k2/8/4K3 w - - 42 99").unwrap();
        assert_eq!(state.halfmove_counter, 42);
        assert_eq!(state.fullmove_number, 99);
    }

    #[test]
    fn missing_counters_default_to_zero_and_one() {
        let state = parse("8/8/8/8/8/5k2/8/4K3 w - -").unwrap();
        assert_eq!(state.halfmove_counter, 0);
        assert_eq!(state.fullmove_number, 1);
    }

    #[test]
    fn rejects_invalid_side() {
        assert!(parse("8/8/8/8/8/8/8/8 x - - 0 1").is_err());
    }

    #[test]
    fn rejects_invalid_en_passant_square() {
        assert!(parse("8/8/8/8/8/8/8/8 w - z9 0 1").is_err());
    }

    #[test]
    fn rejects_truncated_fen() {
        assert!(parse("8/8/8/8/8/8/8/8 w -").is_err());
    }

    #[test]
    fn rejects_invalid_piece_char() {
        assert!(parse("8/8/8/8/8/8/8/7x w - - 0 1").is_err());
    }

    #[test]
    fn rejects_invalid_move_counter() {
        assert!(parse("8/8/8/8/8/8/8/8 w - - abc 1").is_err());
    }

    fn round_trip(fen: &str) {
        assert_eq!(to_fen(parse(fen).unwrap()), fen);
    }

    #[test]
    fn to_fen_round_trips_starting_position() {
        round_trip(START);
    }

    #[test]
    fn to_fen_round_trips_kiwipete() {
        round_trip("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    }

    #[test]
    fn to_fen_counts_empty_squares() {
        round_trip("8/8/8/8/8/8/8/8 w - - 0 1");
    }

    #[test]
    fn to_fen_writes_dash_for_no_rights_and_no_en_passant() {
        round_trip("8/8/8/8/8/5k2/4p3/4K3 b - - 0 1");
    }

    #[test]
    fn to_fen_round_trips_partial_castle_rights() {
        round_trip("r3k2r/8/8/8/8/8/8/R3K2R w KQk - 0 1");
        round_trip("r3k2r/8/8/8/8/8/8/R3K2R w q - 0 1");
    }

    #[test]
    fn to_fen_writes_en_passant_square() {
        round_trip("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
    }
}

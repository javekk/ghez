use crate::game::domain::{Board, Piece, PieceType, Side, Square};
use crate::game::game_state::{CastleRights, GameState};

/// Parse a FEN string into a [`GameState`].
///
/// Only the piece placement, side to move, castling rights, and en passant
/// target are read; the halfmove and fullmove counters are left at their
/// defaults for now.
pub fn parse(fen: &str) -> GameState {
    let fields: Vec<&str> = fen.split(' ').collect();

    let mut state = GameState::new();
    state.board = parse_board(fields[0]);
    state.side = parse_side(fields[1]);
    state.available_castle = parse_castle_rights(fields[2]);
    state.en_passant = parse_en_passant(fields[3]);
    state
}

fn parse_board(placement: &str) -> Board {
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
                board[rank * 8 + file] = Some(parse_piece(piece));
                file += 1;
            }
        }
    }

    board
}

fn parse_piece(symbol: char) -> Piece {
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
        _ => panic!("invalid piece '{symbol}' in FEN"),
    };

    Piece { side, kind }
}

fn parse_side(field: &str) -> Side {
    match field {
        "w" => Side::White,
        "b" => Side::Black,
        _ => panic!("invalid side '{field}' in FEN"),
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

fn parse_en_passant(field: &str) -> Option<Square> {
    match field {
        "-" => None,
        square => Some(square.parse().expect("invalid en passant square in FEN")),
    }
}

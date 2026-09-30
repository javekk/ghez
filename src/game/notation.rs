use crate::game::domain::{Move, PieceType, Side, Square};
use crate::game::game_state::GameState;
use crate::game::movegen;

/// Standard algebraic notation of `mv` without the check/mate suffix.
/// `state` is the position *before* the move is played.
pub fn san_base(state: &GameState, mv: Move, promote_to: PieceType) -> String {
    if mv.is_castle() {
        return if mv.to.file() > mv.from.file() {
            "O-O"
        } else {
            "O-O-O"
        }
        .to_string();
    }

    let is_en_passant = mv.piece.kind == PieceType::Pawn
        && state.en_passant == Some(mv.to)
        && state.get_piece(mv.to).is_none();
    let is_capture = state.get_piece(mv.to).is_some() || is_en_passant;

    let mut out = String::new();
    if mv.piece.kind == PieceType::Pawn {
        if is_capture {
            out.push(file_char(mv.from));
        }
    } else {
        out.push(piece_letter(mv.piece.kind));
        out.push_str(&disambiguation(state, mv));
    }
    if is_capture {
        out.push('x');
    }
    out.push_str(&mv.to.to_string());
    if mv.is_promotion() {
        out.push('=');
        out.push(piece_letter(promote_to));
    }
    out
}

/// "+" or "#" for the position after a move, empty otherwise.
pub fn suffix(state_after: &GameState) -> &'static str {
    if movegen::is_mate(state_after) {
        "#"
    } else if movegen::king_is_in_check(state_after) {
        "+"
    } else {
        ""
    }
}

/// Numbered move-list rows ("1. e4 e5"), one per full move, starting from
/// the position `start`.
pub fn rows(start: &GameState, sans: &[String]) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut number = start.fullmove_number;
    let mut side = start.side;

    for (i, san) in sans.iter().enumerate() {
        match side {
            Side::White => rows.push(format!("{number}. {san}")),
            Side::Black if i == 0 => rows.push(format!("{number}... {san}")),
            Side::Black => {
                if let Some(row) = rows.last_mut() {
                    row.push(' ');
                    row.push_str(san);
                }
            }
        }
        if side == Side::Black {
            number += 1;
        }
        side = side.opponent();
    }
    rows
}

fn disambiguation(state: &GameState, mv: Move) -> String {
    let rivals: Vec<Square> = Square::ALL
        .into_iter()
        .filter(|&sq| sq != mv.from && state.get_piece(sq) == Some(mv.piece))
        .filter(|&sq| movegen::get_legal_moves(state, mv.piece, sq).contains(&mv.to))
        .collect();

    if rivals.is_empty() {
        String::new()
    } else if rivals.iter().all(|sq| sq.file() != mv.from.file()) {
        file_char(mv.from).to_string()
    } else if rivals.iter().all(|sq| sq.rank() != mv.from.rank()) {
        (mv.from.rank() + 1).to_string()
    } else {
        mv.from.to_string()
    }
}

fn file_char(square: Square) -> char {
    (b'a' + square.file() as u8) as char
}

fn piece_letter(kind: PieceType) -> char {
    match kind {
        PieceType::Pawn => 'P',
        PieceType::Knight => 'N',
        PieceType::Bishop => 'B',
        PieceType::Rook => 'R',
        PieceType::Queen => 'Q',
        PieceType::King => 'K',
    }
}

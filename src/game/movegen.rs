use crate::game::domain::{Piece, PieceType, Side, Square};
use crate::game::game_state::GameState;

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

/// Pseudo-legal destinations for a piece, ignoring whether the move leaves the
/// mover's own king in check.
pub fn pseudo_legal_moves(state: &GameState, piece: Piece, from: Square) -> Vec<Square> {
    match piece.kind {
        PieceType::Pawn => pawn_moves(state, from),
        PieceType::Knight => leaper_moves(state, from, &KNIGHT_DELTAS),
        PieceType::King => king_moves(state, from),
        PieceType::Bishop => sliding_moves(state, from, &BISHOP_DIRECTIONS),
        PieceType::Rook => sliding_moves(state, from, &ROOK_DIRECTIONS),
        PieceType::Queen => sliding_moves(state, from, &QUEEN_DIRECTIONS),
    }
}

/// Whether `square` is attacked by the side that is *not* to move in `state`.
pub fn is_square_attacked(state: &GameState, square: Square) -> bool {
    let attacker = state.side.opponent();

    is_attacked_by_pawn(state, square, attacker)
        || is_attacked_by_leaper(state, square, attacker, PieceType::Knight, &KNIGHT_DELTAS)
        || is_attacked_by_leaper(state, square, attacker, PieceType::King, &KING_DELTAS)
        || is_attacked_by_slider(
            state,
            square,
            attacker,
            PieceType::Bishop,
            &BISHOP_DIRECTIONS,
        )
        || is_attacked_by_slider(state, square, attacker, PieceType::Rook, &ROOK_DIRECTIONS)
}

fn leaper_moves(state: &GameState, from: Square, deltas: &[(i8, i8)]) -> Vec<Square> {
    let side = state.side;
    let mut moves = Vec::new();

    for (file_delta, rank_delta) in deltas {
        let Some(target) =
            Square::from_file_rank(from.file() + file_delta, from.rank() + rank_delta)
        else {
            continue;
        };
        let blocked_by_friend = matches!(state.piece_at(target), Some(p) if p.side == side);
        if !blocked_by_friend {
            moves.push(target);
        }
    }
    moves
}

fn sliding_moves(state: &GameState, from: Square, directions: &[(i8, i8)]) -> Vec<Square> {
    let side = state.side;
    let mut moves = Vec::new();

    for (file_step, rank_step) in directions {
        for distance in 1..8 {
            let Some(target) = Square::from_file_rank(
                from.file() + file_step * distance,
                from.rank() + rank_step * distance,
            ) else {
                break;
            };

            match state.piece_at(target) {
                Some(piece) if piece.side == side => break,
                Some(_) => {
                    moves.push(target);
                    break;
                }
                None => moves.push(target),
            }
        }
    }
    moves
}

fn pawn_moves(state: &GameState, from: Square) -> Vec<Square> {
    let side = state.side;
    let file = from.file();
    let rank = from.rank();
    let direction = side.direction();

    let mut moves = Vec::new();

    if let Some(ahead) = Square::from_file_rank(file, rank + direction) {
        if state.piece_at(ahead).is_none() {
            moves.push(ahead);

            let on_start_rank = rank == side.pawn_start_rank();
            if let Some(two_ahead) = Square::from_file_rank(file, rank + 2 * direction) {
                if on_start_rank && state.piece_at(two_ahead).is_none() {
                    moves.push(two_ahead);
                }
            }
        }
    }

    for file_offset in [-1, 1] {
        let Some(target) = Square::from_file_rank(file + file_offset, rank + direction) else {
            continue;
        };

        let is_capture = matches!(state.piece_at(target), Some(p) if p.side != side);
        let is_en_passant = state.en_passant == Some(target);
        if is_capture || is_en_passant {
            moves.push(target);
        }
    }

    moves
}

fn king_moves(state: &GameState, from: Square) -> Vec<Square> {
    let mut moves = leaper_moves(state, from, &KING_DELTAS);
    add_castle_moves(state, from, &mut moves);
    moves
}

/// A castling option: where the king lands, which squares must be vacant, and
/// which squares the king must not cross while under attack.
struct Castle {
    destination: Square,
    empty_squares: &'static [Square],
    safe_squares: &'static [Square],
}

fn add_castle_moves(state: &GameState, king_square: Square, moves: &mut Vec<Square>) {
    if is_square_attacked(state, king_square) {
        return;
    }

    let rights = state.available_castle;
    let (kingside_allowed, queenside_allowed) = match state.side {
        Side::White => (rights.white_kingside, rights.white_queenside),
        Side::Black => (rights.black_kingside, rights.black_queenside),
    };

    let (kingside, queenside) = match state.side {
        Side::White => (
            Castle {
                destination: Square::G1,
                empty_squares: &[Square::F1, Square::G1],
                safe_squares: &[Square::F1, Square::G1],
            },
            Castle {
                destination: Square::C1,
                empty_squares: &[Square::B1, Square::C1, Square::D1],
                safe_squares: &[Square::C1, Square::D1],
            },
        ),
        Side::Black => (
            Castle {
                destination: Square::G8,
                empty_squares: &[Square::F8, Square::G8],
                safe_squares: &[Square::F8, Square::G8],
            },
            Castle {
                destination: Square::C8,
                empty_squares: &[Square::B8, Square::C8, Square::D8],
                safe_squares: &[Square::C8, Square::D8],
            },
        ),
    };

    if kingside_allowed && can_castle(state, &kingside) {
        moves.push(kingside.destination);
    }
    if queenside_allowed && can_castle(state, &queenside) {
        moves.push(queenside.destination);
    }
}

fn can_castle(state: &GameState, castle: &Castle) -> bool {
    let path_is_empty = castle
        .empty_squares
        .iter()
        .all(|&square| state.piece_at(square).is_none());
    let path_is_safe = castle
        .safe_squares
        .iter()
        .all(|&square| !is_square_attacked(state, square));

    path_is_empty && path_is_safe
}

fn is_attacked_by_pawn(state: &GameState, square: Square, attacker: Side) -> bool {
    // A pawn attacks `square` from one rank toward the side to move (i.e. from
    // the direction that pawn would advance to reach it).
    let approach = state.side.direction();
    [-1, 1].iter().any(|&file_offset| {
        Square::from_file_rank(square.file() + file_offset, square.rank() + approach)
            .and_then(|origin| state.piece_at(origin))
            .is_some_and(|piece| piece.kind == PieceType::Pawn && piece.side == attacker)
    })
}

fn is_attacked_by_leaper(
    state: &GameState,
    square: Square,
    attacker: Side,
    kind: PieceType,
    deltas: &[(i8, i8)],
) -> bool {
    deltas.iter().any(|(file_delta, rank_delta)| {
        Square::from_file_rank(square.file() + file_delta, square.rank() + rank_delta)
            .and_then(|origin| state.piece_at(origin))
            .is_some_and(|piece| piece.kind == kind && piece.side == attacker)
    })
}

fn is_attacked_by_slider(
    state: &GameState,
    square: Square,
    attacker: Side,
    kind: PieceType,
    directions: &[(i8, i8)],
) -> bool {
    for (file_step, rank_step) in directions {
        for distance in 1..8 {
            let Some(target) = Square::from_file_rank(
                square.file() + file_step * distance,
                square.rank() + rank_step * distance,
            ) else {
                break;
            };

            let Some(piece) = state.piece_at(target) else {
                continue;
            };

            let is_attacker =
                piece.side == attacker && (piece.kind == kind || piece.kind == PieceType::Queen);
            if is_attacker {
                return true;
            }
            break;
        }
    }
    false
}

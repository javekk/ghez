#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]

pub enum Square {
    A1,
    B1,
    C1,
    D1,
    E1,
    F1,
    G1,
    H1,
    A2,
    B2,
    C2,
    D2,
    E2,
    F2,
    G2,
    H2,
    A3,
    B3,
    C3,
    D3,
    E3,
    F3,
    G3,
    H3,
    A4,
    B4,
    C4,
    D4,
    E4,
    F4,
    G4,
    H4,
    A5,
    B5,
    C5,
    D5,
    E5,
    F5,
    G5,
    H5,
    A6,
    B6,
    C6,
    D6,
    E6,
    F6,
    G6,
    H6,
    A7,
    B7,
    C7,
    D7,
    E7,
    F7,
    G7,
    H7,
    A8,
    B8,
    C8,
    D8,
    E8,
    F8,
    G8,
    H8,
}

impl Square {
    pub fn from_index(v: i8) -> Option<Square> {
        if (0..64).contains(&v) {
            Some(unsafe { std::mem::transmute::<u8, Square>(v as u8) })
        } else {
            None
        }
    }

    pub fn file(self) -> i8 {
        (self as i8) % 8
    }

    pub fn rank(self) -> i8 {
        (self as i8) / 8
    }

    pub fn from_file_rank(file: i8, rank: i8) -> Option<Square> {
        if !(0..8).contains(&file) || !(0..8).contains(&rank) {
            return None;
        }
        Square::from_index(rank * 8 + file)
    }
}

pub type Board = [Option<Piece>; 64];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn direction(self) -> i8 {
        match self {
            Side::White => 1,
            Side::Black => -1,
        }
    }

    pub fn pawn_start_rank(self) -> i8 {
        match self {
            Side::White => 1,
            Side::Black => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub side: Side,
    pub kind: PieceType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub piece: Piece,
    pub from: Square,
    pub to: Square,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_index_returns_square_for_valid_range() {
        assert_eq!(Square::from_index(0), Some(Square::A1));
        assert_eq!(Square::from_index(63), Some(Square::H8));
        assert_eq!(Square::from_index(8), Some(Square::A2));
    }

    #[test]
    fn from_index_returns_none_for_out_of_range() {
        assert_eq!(Square::from_index(-1), None);
        assert_eq!(Square::from_index(64), None);
    }

    #[test]
    fn square_as_usize_round_trips_through_from_index() {
        for i in 0..64i8 {
            let sq = Square::from_index(i).unwrap();
            assert_eq!(sq as i8, i);
        }
    }
}

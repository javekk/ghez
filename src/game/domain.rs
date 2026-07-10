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

impl std::str::FromStr for Square {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().trim() {
            "A1" => Ok(Square::A1),
            "B1" => Ok(Square::B1),
            "C1" => Ok(Square::C1),
            "D1" => Ok(Square::D1),
            "E1" => Ok(Square::E1),
            "F1" => Ok(Square::F1),
            "G1" => Ok(Square::G1),
            "H1" => Ok(Square::H1),
            "A2" => Ok(Square::A2),
            "B2" => Ok(Square::B2),
            "C2" => Ok(Square::C2),
            "D2" => Ok(Square::D2),
            "E2" => Ok(Square::E2),
            "F2" => Ok(Square::F2),
            "G2" => Ok(Square::G2),
            "H2" => Ok(Square::H2),
            "A3" => Ok(Square::A3),
            "B3" => Ok(Square::B3),
            "C3" => Ok(Square::C3),
            "D3" => Ok(Square::D3),
            "E3" => Ok(Square::E3),
            "F3" => Ok(Square::F3),
            "G3" => Ok(Square::G3),
            "H3" => Ok(Square::H3),
            "A4" => Ok(Square::A4),
            "B4" => Ok(Square::B4),
            "C4" => Ok(Square::C4),
            "D4" => Ok(Square::D4),
            "E4" => Ok(Square::E4),
            "F4" => Ok(Square::F4),
            "G4" => Ok(Square::G4),
            "H4" => Ok(Square::H4),
            "A5" => Ok(Square::A5),
            "B5" => Ok(Square::B5),
            "C5" => Ok(Square::C5),
            "D5" => Ok(Square::D5),
            "E5" => Ok(Square::E5),
            "F5" => Ok(Square::F5),
            "G5" => Ok(Square::G5),
            "H5" => Ok(Square::H5),
            "A6" => Ok(Square::A6),
            "B6" => Ok(Square::B6),
            "C6" => Ok(Square::C6),
            "D6" => Ok(Square::D6),
            "E6" => Ok(Square::E6),
            "F6" => Ok(Square::F6),
            "G6" => Ok(Square::G6),
            "H6" => Ok(Square::H6),
            "A7" => Ok(Square::A7),
            "B7" => Ok(Square::B7),
            "C7" => Ok(Square::C7),
            "D7" => Ok(Square::D7),
            "E7" => Ok(Square::E7),
            "F7" => Ok(Square::F7),
            "G7" => Ok(Square::G7),
            "H7" => Ok(Square::H7),
            "A8" => Ok(Square::A8),
            "B8" => Ok(Square::B8),
            "C8" => Ok(Square::C8),
            "D8" => Ok(Square::D8),
            "E8" => Ok(Square::E8),
            "F8" => Ok(Square::F8),
            "G8" => Ok(Square::G8),
            "H8" => Ok(Square::H8),
            _ => panic!("Invalid en passant square"),
        }
    }
}

impl Square {
    /// Every square in board order (a1..h1, a2..h2, ..., a8..h8).
    pub const ALL: [Square; 64] = {
        use Square::*;
        [
            A1, B1, C1, D1, E1, F1, G1, H1, //
            A2, B2, C2, D2, E2, F2, G2, H2, //
            A3, B3, C3, D3, E3, F3, G3, H3, //
            A4, B4, C4, D4, E4, F4, G4, H4, //
            A5, B5, C5, D5, E5, F5, G5, H5, //
            A6, B6, C6, D6, E6, F6, G6, H6, //
            A7, B7, C7, D7, E7, F7, G7, H7, //
            A8, B8, C8, D8, E8, F8, G8, H8,
        ]
    };

    pub fn from_index(index: i8) -> Option<Square> {
        usize::try_from(index)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
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
    pub fn opponent(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }

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

impl Move {
    pub fn is_pawn_double_push(&self) -> bool {
        if self.piece.kind == PieceType::Pawn {
            (self.from.rank() - self.to.rank()).abs() == 2
        } else {
            false
        }
    }

    pub fn is_castle(&self) -> bool {
        if self.piece.kind != PieceType::King {
            return false;
        }

        matches!(
            (self.from, self.to),
            (Square::E1, Square::G1)
                | (Square::E1, Square::C1)
                | (Square::E8, Square::G8)
                | (Square::E8, Square::C8)
        )
    }
}

pub const INITIAL_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 ";
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
pub const EASY_POSITION: &str = "8/8/8/8/8/5k2/4p3/4K3 b - - 0 1";

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

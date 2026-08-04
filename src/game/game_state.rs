use crate::game::domain::{
    Board, CastleRights, Piece,
    Side::{self},
    Square,
};

pub enum DrawReason {
    Stalemate,
    FiftyMoveRule,
    ThreefoldRepetition,
    InsufficientMaterial,
    Agreement,
}

pub enum GameStatus {
    Chilling,
    Battling,
    Draw(DrawReason),
    Mated(Side),      //  which side has lost
    LostOnTime(Side), //  which side has lost
    RunAway(Side),    //  which side has given up
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameState {
    pub board: Board,
    pub side: Side,
    pub en_passant: Option<Square>,

    pub available_castle: CastleRights,

    pub halfmove_counter: i16,
    pub fullmove_number: i16,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            board: [None; 64],
            side: Side::White,
            en_passant: None,
            available_castle: CastleRights {
                white_kingside: true,
                white_queenside: true,
                black_kingside: true,
                black_queenside: true,
            },
            halfmove_counter: 0,
            fullmove_number: 1,
        }
    }

    pub fn get_repetion_key(&self) -> (Board, Side, Option<Square>, CastleRights) {
        (
            self.board,
            self.side,
            self.en_passant,
            self.available_castle,
        )
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        self.board[square as usize]
    }

    pub fn set_piece(&mut self, square: Square, piece: Piece) {
        self.board[square as usize] = Some(piece);
    }

    pub fn clear_square(&mut self, square: Square) -> bool {
        if self.get_piece(square).is_none() {
            return false;
        };

        self.board[square as usize] = None;
        true
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.board[square as usize]
    }

    pub fn move_piece(&mut self, from: Square, to: Square) -> bool {
        let Some(piece) = self.get_piece(from) else {
            return false;
        };

        self.set_piece(to, piece);
        self.clear_square(from);
        true
    }
}

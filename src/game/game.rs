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
                // Disegna i puntini per i suggerimenti di mossa
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
}

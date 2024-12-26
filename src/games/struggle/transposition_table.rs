use dashmap::DashMap;

use super::board::{Board, PiecePosition};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoardHash(u64);

#[derive(Default)]
pub struct TranspositionTable {
    table: DashMap<BoardHash, f32>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: DashMap::new(),
        }
    }

    pub fn get(&self, board: BoardHash) -> Option<f32> {
        self.table.get(&board).map(|entry| *entry.value())
    }

    pub fn insert(&self, board: BoardHash, value: f32) {
        self.table.insert(board, value);
    }
}

// We can pack the board state into a single 64-bit integer
// There are 28 board slots, 4 goal slots, and 2 players with 4 pieces each
// We can store the location of each piece with 5 bits
// Additionally, we need one more bit indicating if the piece is on the board
// bits 0-0:   player 0 piece 0 on board
// bits 1-5:   player 0 piece 0 location
// bits 6-6:   player 0 piece 1 on board
// bits 7-11:  player 0 piece 1 location
// bits 12-12: player 0 piece 2 on board
// bits 13-17: player 0 piece 2 location
// bits 18-18: player 0 piece 3 on board
// bits 19-23: player 0 piece 3 location
// bits 24-24: player 1 piece 0 on board
// bits 25-29: player 1 piece 0 location
// bits 30-30: player 1 piece 1 on board
// bits 31-35: player 1 piece 1 location
// bits 36-36: player 1 piece 2 on board
// bits 37-41: player 1 piece 2 location
// bits 42-42: player 1 piece 3 on board
// bits 43-47: player 1 piece 3 location
// bits 48-63: unused

pub fn get_board_hash(board: &Board) -> BoardHash {
    let mut packed = 0u64;

    const PLAYER_0_OFFSET: u64 = 0;

    for (piece_index, piece) in board.piece_cache.0.iter().enumerate() {
        let piece_offset = PLAYER_0_OFFSET + (piece_index as u64) * 6;
        // Set the first bit to 1 if the piece is on the board
        packed |= 1 << piece_offset;

        match piece {
            PiecePosition::Board(board_index) => {
                debug_assert!(*board_index < 28);
                packed |= (*board_index as u64) << piece_offset + 1;
            }
            PiecePosition::Goal(goal_index) => {
                debug_assert!(*goal_index < 4);
                packed |= (28 + *goal_index as u64) << piece_offset + 1;
            }
        }
    }

    const PLAYER_1_OFFSET: u64 = 24;

    for (piece_index, piece) in board.piece_cache.1.iter().enumerate() {
        let piece_offset = PLAYER_1_OFFSET + (piece_index as u64) * 6;
        packed |= 1 << piece_offset;

        match piece {
            PiecePosition::Board(board_index) => {
                debug_assert!(*board_index < 28);
                packed |= (*board_index as u64) << piece_offset + 1;
            }
            PiecePosition::Goal(goal_index) => {
                debug_assert!(*goal_index < 4);
                packed |= (28 + *goal_index as u64) << piece_offset + 1;
            }
        }
    }

    // We don't need to store how many pieces are waiting, because it's implied by the board state

    BoardHash(packed)
}

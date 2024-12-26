use dashmap::DashMap;
use rustc_hash::FxBuildHasher;

use super::{
    board::{Board, PiecePosition},
    PlayerColor,
};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoardHash(u64);

#[derive(Debug, Clone, Default)]
struct TranspositionTableEntry {
    pub value: f32,
    pub depth: u8,
}

#[derive(Default)]
pub struct TranspositionTable {
    table: DashMap<BoardHash, TranspositionTableEntry, FxBuildHasher>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: DashMap::default(),
        }
    }

    pub fn get(&self, board_hash: BoardHash, depth: u8) -> Option<f32> {
        self.table
            .get(&board_hash)
            .filter(|entry| {
                // Only return the value if the depth is greater or equal to the depth of the entry
                entry.depth >= depth
            })
            .map(|entry| entry.value)
    }

    pub fn insert_if_better(&self, board_hash: BoardHash, value: f32, depth: u8) {
        self.table
            .entry(board_hash)
            .and_modify(|entry| {
                // If the new depth is greater than the current depth, update the entry
                if depth > entry.depth {
                    entry.value = value;
                    entry.depth = depth;
                }
            })
            .or_insert_with(|| TranspositionTableEntry { value, depth });
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
// bits 48-48: current player (0 or 1)
// bits 49-63: unused

const STORE_CURRENT_PLAYER_IN_KEY: bool = false;

pub fn get_board_hash(board: &Board, current_player: PlayerColor) -> BoardHash {
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

    if STORE_CURRENT_PLAYER_IN_KEY {
        let current_player_bit = if current_player == board.players.0 {
            0
        } else {
            1
        };

        packed |= current_player_bit << 48;
    }

    BoardHash(packed)
}

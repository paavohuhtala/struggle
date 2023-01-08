use std::ops::Range;

use arrayvec::ArrayVec;
use rand::Rng;
use tinyvec::TinyVec;

use crate::games::struggle::{
    board::{BoardCell, HomeBase, PiecePosition},
    PlayerColor, COLORS,
};

type TwistGoal = [BoardCell; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwistRotation {
    Initial = 0,
    Ccw90,
    Ccw180,
    Ccw270,
}

impl TwistRotation {
    pub const fn next(self) -> Self {
        match self {
            Self::Initial => Self::Ccw90,
            Self::Ccw90 => Self::Ccw180,
            Self::Ccw180 => Self::Ccw270,
            Self::Ccw270 => Self::Initial,
        }
    }

    pub const fn to_offset(self) -> u8 {
        match self {
            Self::Initial => 0,
            Self::Ccw90 => 24,
            Self::Ccw180 => 16,
            Self::Ccw270 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpinSection {
    RedToBlue,
    BlueToYellow,
    YellowToGreen,
    GreenToRed,
}

impl SpinSection {
    pub const ALL: [Self; 4] = [
        Self::RedToBlue,
        Self::BlueToYellow,
        Self::YellowToGreen,
        Self::GreenToRed,
    ];
}

pub type TwistPieceVec = ArrayVec<PiecePosition, 4>;

#[derive(Clone)]
pub struct TwistBoard {
    pub tiles: [BoardCell; TwistBoard::TILES],
    pub goals: [TwistGoal; 4],
    pub home_bases: [HomeBase; 4],
    pub rotation: TwistRotation,

    players: (PlayerColor, PlayerColor),
    piece_cache: (TwistPieceVec, TwistPieceVec),
}

impl TwistBoard {
    pub const TILES: usize = 8 * 4;

    pub const RED_START: u8 = 0;
    pub const BLUE_START: u8 = Self::RED_START + 8;
    pub const YELLOW_START: u8 = Self::BLUE_START + 8;
    pub const GREEN_START: u8 = Self::YELLOW_START + 8;

    pub const STARTS: [u8; 4] = [
        Self::RED_START,
        Self::BLUE_START,
        Self::YELLOW_START,
        Self::GREEN_START,
    ];

    const BASE_GOAL_ENTER: [u8; 4] = [
        (Self::TILES as u8) - 1,
        Self::BLUE_START - 1,
        Self::YELLOW_START - 1,
        Self::GREEN_START - 1,
    ];

    pub fn new(players: (PlayerColor, PlayerColor)) -> Self {
        let board = Self {
            tiles: [None; Self::TILES],
            goals: [[None; 3]; 4],
            home_bases: COLORS.map(|_| HomeBase::new()),
            rotation: TwistRotation::Initial,
            players,
            piece_cache: (TwistPieceVec::new(), TwistPieceVec::new()),
        };

        board
    }

    const fn internal_get_goal_entry(rotation: TwistRotation, color: PlayerColor) -> u8 {
        let offset = rotation.to_offset();
        let goal = Self::BASE_GOAL_ENTER[color as usize];
        (goal + offset) % (Self::TILES as u8)
    }

    // Static lookup table for goal entrances for each rotation
    const GOAL_ENTRIES: [[u8; 4]; 4] = [
        [
            Self::internal_get_goal_entry(TwistRotation::Initial, PlayerColor::Red),
            Self::internal_get_goal_entry(TwistRotation::Initial, PlayerColor::Blue),
            Self::internal_get_goal_entry(TwistRotation::Initial, PlayerColor::Yellow),
            Self::internal_get_goal_entry(TwistRotation::Initial, PlayerColor::Green),
        ],
        [
            Self::internal_get_goal_entry(TwistRotation::Ccw90, PlayerColor::Red),
            Self::internal_get_goal_entry(TwistRotation::Ccw90, PlayerColor::Blue),
            Self::internal_get_goal_entry(TwistRotation::Ccw90, PlayerColor::Yellow),
            Self::internal_get_goal_entry(TwistRotation::Ccw90, PlayerColor::Green),
        ],
        [
            Self::internal_get_goal_entry(TwistRotation::Ccw180, PlayerColor::Red),
            Self::internal_get_goal_entry(TwistRotation::Ccw180, PlayerColor::Blue),
            Self::internal_get_goal_entry(TwistRotation::Ccw180, PlayerColor::Yellow),
            Self::internal_get_goal_entry(TwistRotation::Ccw180, PlayerColor::Green),
        ],
        [
            Self::internal_get_goal_entry(TwistRotation::Ccw270, PlayerColor::Red),
            Self::internal_get_goal_entry(TwistRotation::Ccw270, PlayerColor::Blue),
            Self::internal_get_goal_entry(TwistRotation::Ccw270, PlayerColor::Yellow),
            Self::internal_get_goal_entry(TwistRotation::Ccw270, PlayerColor::Green),
        ],
    ];

    pub const fn get_goal_entrance(rotation: TwistRotation, color: PlayerColor) -> u8 {
        Self::GOAL_ENTRIES[rotation as usize][color as usize]
    }

    pub const fn get_start(color: PlayerColor) -> u8 {
        match color {
            PlayerColor::Red => Self::RED_START,
            PlayerColor::Blue => Self::BLUE_START,
            PlayerColor::Yellow => Self::YELLOW_START,
            PlayerColor::Green => Self::GREEN_START,
        }
    }

    pub fn get_winner(&self) -> Option<PlayerColor> {
        for (i, goal) in self.goals.iter().enumerate() {
            let player = PlayerColor::from(i);
            let goal_entry_pos = Self::get_goal_entrance(self.rotation, player);

            if self.tiles[goal_entry_pos as usize] == Some(player)
                && goal.iter().all(|&x| x == Some(player))
            {
                return Some(player);
            }
        }

        None
    }

    pub const fn get_spin_section_range(spin_section: SpinSection) -> Range<usize> {
        let start = match spin_section {
            SpinSection::RedToBlue => Self::RED_START,
            SpinSection::BlueToYellow => Self::BLUE_START,
            SpinSection::YellowToGreen => Self::YELLOW_START,
            SpinSection::GreenToRed => Self::GREEN_START,
        } + 1;

        let end = start + 5;

        start as usize..end as usize
    }

    pub fn get_spin_section(&self, spin_section: SpinSection) -> &[BoardCell; 5] {
        let range = Self::get_spin_section_range(spin_section);
        <&[BoardCell; 5]>::try_from(&self.tiles[range]).unwrap()
    }

    pub fn get_spin_section_mut(&mut self, spin_section: SpinSection) -> &mut [BoardCell; 5] {
        let range = Self::get_spin_section_range(spin_section);
        <&mut [BoardCell; 5]>::try_from(&mut self.tiles[range]).unwrap()
    }

    pub fn rotate_spin_section(&mut self, spin_section: SpinSection) {
        self.get_spin_section_mut(spin_section).reverse();
    }

    pub fn perform_move(&mut self, player: PlayerColor, mov: &TwistMove) {
        match &mov.0 {
            NumberDieMove::MovePiece { from, to, eats } => {
                if *eats {
                    let target_player = self.tiles[*to as usize]
                        .expect("Player should have a piece in target position");

                    self.home_bases[target_player as usize].add_piece();
                }

                self.tiles[*to as usize] = Some(player);

                match from {
                    MoveFrom::Home => {
                        self.home_bases[player as usize]
                            .remove_piece()
                            .expect("Player should have a piece in home base");
                    }
                    MoveFrom::Board(pos) => {
                        assert_eq!(self.tiles[*pos as usize], Some(player));
                        self.tiles[*pos as usize] = None;
                    }
                }
            }
            NumberDieMove::MoveToGoal {
                from_board,
                to_goal,
            } => {
                self.goals[player as usize][*to_goal as usize] = Some(player);
                self.tiles[*from_board as usize] = None;
            }
            NumberDieMove::DoNothing => {}
        }

        match &mov.1 {
            ActionDieMove::SpinSection(section) => {
                self.rotate_spin_section(*section);
            }
            ActionDieMove::RotateBoard => {
                self.rotation = self.rotation.next();
            }
            ActionDieMove::DoNothing => {}
        }

        self.update_piece_cache();
    }

    fn get_pieces_internal(
        &self,
        player: PlayerColor,
        enemy: PlayerColor,
    ) -> (TwistPieceVec, TwistPieceVec) {
        let mut player_pieces = TwistPieceVec::new_const();
        let mut enemy_pieces = TwistPieceVec::new_const();

        for (i, &tile) in self.tiles.iter().enumerate() {
            match tile {
                Some(tile_player) if tile_player == player => {
                    player_pieces.push(PiecePosition::Board(i as u8));
                }
                Some(_) => {
                    enemy_pieces.push(PiecePosition::Board(i as u8));
                }
                None => {}
            }
        }

        for (i, piece) in self.goals[player as usize].iter().enumerate() {
            if piece.is_some() {
                player_pieces.push(PiecePosition::Goal(i as u8));
            }
        }

        for (i, piece) in self.goals[enemy as usize].iter().enumerate() {
            if piece.is_some() {
                enemy_pieces.push(PiecePosition::Goal(i as u8));
            }
        }

        (player_pieces, enemy_pieces)
    }

    pub(crate) fn update_piece_cache(&mut self) {
        self.piece_cache = self.get_pieces_internal(self.players.0, self.players.1);
    }

    pub fn get_pieces(&self, player: PlayerColor) -> (&TwistPieceVec, &TwistPieceVec) {
        if player == self.players.0 {
            (&self.piece_cache.0, &self.piece_cache.1)
        } else {
            (&self.piece_cache.1, &self.piece_cache.0)
        }
    }

    pub fn clockwise_distance(from: u8, to: u8) -> u8 {
        if to >= from {
            to - from
        } else {
            Self::TILES as u8 - from + to
        }
    }

    pub fn distance_to_goal(&self, player: PlayerColor, pos: u8) -> u8 {
        let goal = Self::get_goal_entrance(self.rotation, player);
        Self::clockwise_distance(pos, goal)
    }

    pub fn update(&mut self, updater: impl FnOnce(&mut TwistBoard)) {
        updater(self);
        self.update_piece_cache();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionDie {
    #[default]
    DoNothing,
    SpinSection,
    RotateBoard,
}

impl ActionDie {
    pub fn get_random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..6) {
            0 | 1 | 2 => Self::DoNothing,
            3 | 4 => Self::SpinSection,
            5 => Self::RotateBoard,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DieResult {
    pub number: u8,
    pub action: ActionDie,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveFrom {
    Home,
    Board(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumberDieMove {
    DoNothing,
    MovePiece { from: MoveFrom, to: u8, eats: bool },
    MoveToGoal { from_board: u8, to_goal: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionDieMove {
    DoNothing,
    SpinSection(SpinSection),
    RotateBoard,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TwistMove(pub NumberDieMove, pub ActionDieMove);

impl Default for TwistMove {
    fn default() -> Self {
        Self(NumberDieMove::DoNothing, ActionDieMove::DoNothing)
    }
}

// Store up to 4 moves inline
pub type TwistMoveVec = TinyVec<[TwistMove; 8]>;

#[cfg(test)]
mod tests {
    use super::*;

    const P1: PlayerColor = PlayerColor::Red;
    const P2: PlayerColor = PlayerColor::Yellow;

    #[test]
    fn get_winner_goal_filled() {
        let mut board = TwistBoard::new((P1, P2));

        board.update(|board| {
            board.goals[0][0] = Some(P1);
            board.goals[0][1] = Some(P1);
            board.goals[0][2] = Some(P1);
        });

        // Winning requires all goals AND the goal entrance to be filled
        assert_eq!(board.get_winner(), None);
    }

    #[test]
    fn get_winner_goal_entrance_filled() {
        let mut board = TwistBoard::new((P1, P2));

        board.update(|board| {
            board.goals[0][0] = Some(P1);
            board.goals[0][1] = Some(P1);
            board.goals[0][2] = Some(P1);
            board.tiles[TwistBoard::get_goal_entrance(TwistRotation::Initial, P1) as usize] =
                Some(P1);
        });

        assert_eq!(board.get_winner(), Some(P1));
    }

    #[test]
    fn get_winner_goal_entrance_filled_yellow() {
        let mut board = TwistBoard::new((P1, P2));

        board.update(|board| {
            board.goals[2][0] = Some(P2);
            board.goals[2][1] = Some(P2);
            board.goals[2][2] = Some(P2);
            board.tiles[TwistBoard::get_goal_entrance(TwistRotation::Initial, P2) as usize] =
                Some(P2);
        });

        assert_eq!(board.get_winner(), Some(P2));
    }

    #[test]
    fn get_winner_goal_entrance_filled_other_player() {
        let mut board = TwistBoard::new((P1, P2));

        board.update(|board| {
            board.goals[0][0] = Some(P1);
            board.goals[0][1] = Some(P1);
            board.goals[0][2] = Some(P1);

            board.tiles[TwistBoard::get_goal_entrance(TwistRotation::Initial, P1) as usize] =
                Some(P2);
        });

        assert_eq!(board.get_winner(), None);
    }

    #[test]
    fn get_winner_rotate() {
        let mut board = TwistBoard::new((P1, P2));

        board.update(|board| {
            board.rotation = TwistRotation::Ccw90;

            board.goals[0][0] = Some(P1);
            board.goals[0][1] = Some(P1);
            board.goals[0][2] = Some(P1);

            board.tiles[TwistBoard::get_goal_entrance(TwistRotation::Initial, P1) as usize] =
                Some(P1);
        });

        assert_eq!(board.get_winner(), None);

        board.update(|board| {
            board.rotation = TwistRotation::Initial;
        });

        assert_eq!(board.get_winner(), Some(PlayerColor::Red));
    }
}

use std::borrow::Cow;

use arrayvec::ArrayVec;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Player {
    Red = 0,
    Blue,
    Yellow,
    Green,
}

pub const COLORS: [Player; 4] = [Player::Red, Player::Blue, Player::Yellow, Player::Green];

type BoardCell = Option<Player>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiecePosition {
    Board(u8),
    Goal(u8),
}

impl PiecePosition {
    pub fn as_board_index(self) -> Option<u8> {
        match self {
            PiecePosition::Board(index) => Some(index),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Board {
    pub tiles: [BoardCell; 7 * 4],
    pub goals: [Goal; 4],
    pub home_bases: [HomeBase; 4],

    players: (Player, Player),
    piece_cache: (PieceVec, PieceVec),
}

pub type MoveVec = ArrayVec<ValidMove, 4>;
pub type PieceVec = ArrayVec<PiecePosition, 4>;

impl Board {
    pub fn new(player_a: Player, player_b: Player) -> Self {
        Board {
            tiles: [None; 7 * 4],
            goals: COLORS.map(|_| [None; 4]),
            home_bases: COLORS.map(|_| HomeBase::new()),

            players: (player_a, player_b),
            piece_cache: (PieceVec::new(), PieceVec::new()),
        }
    }
}

impl Board {
    pub const RED_START: u8 = 0;
    pub const BLUE_START: u8 = Self::RED_START + 7;
    pub const YELLOW_START: u8 = Self::BLUE_START + 7;
    pub const GREEN_START: u8 = Self::YELLOW_START + 7;

    pub fn get_start(player: Player) -> u8 {
        match player {
            Player::Red => Self::RED_START,
            Player::Blue => Self::BLUE_START,
            Player::Yellow => Self::YELLOW_START,
            Player::Green => Self::GREEN_START,
        }
    }

    pub fn get_winner(&self) -> Option<Player> {
        self.goals.iter().find_map(|g| {
            let all_filled = g.iter().all(|cell| cell.is_some());
            if all_filled {
                Some(g[0].unwrap())
            } else {
                None
            }
        })
    }

    pub fn get_pieces(&self, player: Player, _enemy: Player) -> (&PieceVec, &PieceVec) {
        if player == self.players.0 {
            (&self.piece_cache.0, &self.piece_cache.1)
        } else {
            (&self.piece_cache.1, &self.piece_cache.0)
        }
    }

    fn get_pieces_internal(&self, player: Player, enemy: Player) -> (PieceVec, PieceVec) {
        let mut player_positions = PieceVec::new_const();
        let mut enemy_positions = PieceVec::new_const();

        for (i, piece) in self.tiles.iter().enumerate() {
            match piece {
                Some(color) if *color == player => {
                    player_positions.push(PiecePosition::Board(i as u8))
                }
                Some(_) => enemy_positions.push(PiecePosition::Board(i as u8)),
                _ => {}
            }
        }

        let player_goal = &self.goals[player as usize];

        for (i, piece) in player_goal.iter().enumerate() {
            if piece.is_some() {
                player_positions.push(PiecePosition::Goal(i as u8))
            }
        }

        let enemy_goal = &self.goals[enemy as usize];

        for (i, piece) in enemy_goal.iter().enumerate() {
            if piece.is_some() {
                enemy_positions.push(PiecePosition::Goal(i as u8))
            }
        }

        (player_positions, enemy_positions)
    }

    pub fn get_moves(&self, dice: u8, player: Player, enemy: Player) -> MoveVec {
        let mut moves = MoveVec::new_const();

        let home_base = &self.home_bases[player as usize];
        let goal = &self.goals[player as usize];
        let (pieces, _) = self.get_pieces(player, enemy);
        let player_start = Self::get_start(player);

        if home_base.pieces_waiting > 0 && dice == 6 {
            match self.tiles[player_start as usize] {
                Some(other_piece) if other_piece != player => {
                    moves.push(ValidMove::AddNewPiece { eats: true });
                }
                None => {
                    moves.push(ValidMove::AddNewPiece { eats: false });
                }
                _ => {}
            }
        }

        for piece in pieces {
            match piece {
                PiecePosition::Board(current_pos) => {
                    let current_pos = *current_pos;
                    let new_pos = (current_pos + dice) % self.tiles.len() as u8;

                    let goal_relative_pos = match player as usize {
                        0 => {
                            // went around
                            if new_pos < current_pos {
                                Some(new_pos)
                            } else {
                                None
                            }
                        }
                        _ => {
                            if current_pos < player_start && new_pos >= player_start {
                                Some(new_pos - player_start)
                            } else {
                                None
                            }
                        }
                    };

                    match goal_relative_pos {
                        Some(pos) => {
                            if let Some(None) = goal.get(pos as usize) {
                                moves.push(ValidMove::MoveToGoal {
                                    from_board: current_pos,
                                    to_goal: pos,
                                });
                            }
                        }
                        None => match self.tiles[new_pos as usize] {
                            None => {
                                moves.push(ValidMove::MovePiece {
                                    from: current_pos,
                                    to: new_pos,
                                    eats: false,
                                });
                            }
                            Some(other_piece) if other_piece != player => {
                                moves.push(ValidMove::MovePiece {
                                    from: current_pos,
                                    to: new_pos,
                                    eats: true,
                                });
                            }
                            _ => {}
                        },
                    }
                }
                PiecePosition::Goal(i) => {
                    let new_pos = i + dice;

                    if let Some(None) = goal.get(new_pos as usize) {
                        moves.push(ValidMove::MoveInGoal {
                            from_goal: *i,
                            to_goal: new_pos,
                        });
                    }
                }
            }
        }

        if moves.is_empty() {
            moves.push(ValidMove::SkipTurn);
        }

        moves
    }

    pub fn perform_move(&mut self, player: Player, mov: &ValidMove) {
        match mov {
            ValidMove::AddNewPiece { eats } => {
                let start = Self::get_start(player);

                if *eats {
                    let other_player =
                        self.tiles[start as usize].expect("expected enemy piece at start");
                    self.home_bases[other_player as usize].add_piece();
                }

                self.tiles[start as usize] = Some(player);
                self.home_bases[player as usize]
                    .remove_piece()
                    .expect("Player should have pieces left in home base");
            }
            ValidMove::MovePiece { from, to, eats } => {
                if *eats {
                    let target_player = self.tiles[*to as usize]
                        .expect("expecting eating move to have piece in target");
                    self.home_bases[target_player as usize].add_piece();
                }

                self.tiles[*to as usize] = self.tiles[*from as usize];
                self.tiles[*from as usize] = None;
            }
            ValidMove::MoveToGoal {
                from_board,
                to_goal,
            } => {
                self.goals[player as usize][*to_goal as usize] = self.tiles[*from_board as usize];
                self.tiles[*from_board as usize] = None;
            }
            ValidMove::MoveInGoal { from_goal, to_goal } => {
                self.goals[player as usize][*to_goal as usize] =
                    self.goals[player as usize][*from_goal as usize];
                self.goals[player as usize][*from_goal as usize] = None;
            }
            ValidMove::SkipTurn => {}
        }

        self.update_piece_cache();
    }

    pub fn update_piece_cache(&mut self) {
        self.piece_cache = self.get_pieces_internal(self.players.0, self.players.1);
    }

    pub fn with_move(&self, player: Player, mov: &ValidMove) -> Cow<'_, Self> {
        match mov {
            ValidMove::SkipTurn => Cow::Borrowed(self),
            otherwise => {
                let mut board = self.clone();
                board.perform_move(player, otherwise);
                Cow::Owned(board)
            }
        }
    }

    pub fn clockwise_distance(&self, from: u8, to: u8) -> u8 {
        if to >= from {
            to - from
        } else {
            self.tiles.len() as u8 - from + to
        }
    }

    pub fn distance_to_goal(&self, player: Player, pos: u8) -> u8 {
        let goal = match player {
            Player::Red => 27,
            _ => Self::get_start(player) - 1,
        };

        self.clockwise_distance(pos, goal)
    }
}

#[derive(Debug, Clone)]
pub struct HomeBase {
    pub pieces_waiting: u8,
}

impl HomeBase {
    fn new() -> HomeBase {
        HomeBase { pieces_waiting: 4 }
    }

    pub fn remove_piece(&mut self) -> Option<()> {
        if self.pieces_waiting > 0 {
            self.pieces_waiting -= 1;
            Some(())
        } else {
            None
        }
    }

    pub fn add_piece(&mut self) {
        self.pieces_waiting += 1;
    }
}

type Goal = [BoardCell; 4];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidMove {
    AddNewPiece { eats: bool },
    MovePiece { from: u8, to: u8, eats: bool },
    MoveToGoal { from_board: u8, to_goal: u8 },
    MoveInGoal { from_goal: u8, to_goal: u8 },
    SkipTurn,
}

impl ValidMove {
    pub fn eats(&self) -> bool {
        match self {
            ValidMove::AddNewPiece { eats } => *eats,
            ValidMove::MovePiece { eats, .. } => *eats,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_goal_move_1() {
        let mut board = Board::new(Player::Red, Player::Yellow);
        board.tiles[27] = Some(Player::Red);
        board.update_piece_cache();
        let moves = board.get_moves(1, Player::Red, Player::Yellow);

        assert_eq!(moves.len(), 1);
        assert_eq!(
            moves[0],
            ValidMove::MoveToGoal {
                from_board: 27,
                to_goal: 0
            }
        );

        board.perform_move(Player::Red, &moves[0]);

        assert_eq!(board.tiles[27], None);
        assert_eq!(board.goals[Player::Red as usize][0], Some(Player::Red));
    }

    #[test]
    fn red_goal_move_2() {
        let mut board = Board::new(Player::Red, Player::Yellow);
        board.tiles[26] = Some(Player::Red);
        board.update_piece_cache();
        let moves = board.get_moves(2, Player::Red, Player::Yellow);

        assert_eq!(moves.len(), 1);
        assert_eq!(
            moves[0],
            ValidMove::MoveToGoal {
                from_board: 26,
                to_goal: 0
            }
        );

        board.perform_move(Player::Red, &moves[0]);

        assert_eq!(board.tiles[27], None);
        assert_eq!(board.goals[Player::Red as usize][0], Some(Player::Red));
    }

    #[test]
    fn yellow_move_around_red_home() {
        let mut board = Board::new(Player::Red, Player::Yellow);
        board.tiles[27] = Some(Player::Yellow);
        board.update_piece_cache();
        let moves = board.get_moves(1, Player::Yellow, Player::Red);

        assert_eq!(moves.len(), 1);
        assert_eq!(
            moves[0],
            ValidMove::MovePiece {
                from: 27,
                to: 0,
                eats: false
            }
        );

        board.perform_move(Player::Yellow, &moves[0]);

        assert_eq!(board.tiles[27], None);
        assert_eq!(board.tiles[0], Some(Player::Yellow));
    }

    #[test]
    fn yellow_distance_to_goal() {
        let board = Board::new(Player::Red, Player::Yellow);
        assert_eq!(board.distance_to_goal(Player::Yellow, 0), 13);
        assert_eq!(board.distance_to_goal(Player::Yellow, 27), 14);
        assert_eq!(board.distance_to_goal(Player::Yellow, 13), 0);
        assert_eq!(board.distance_to_goal(Player::Yellow, 14), 27);
    }

    #[test]
    fn red_distance_to_goal() {
        let board = Board::new(Player::Red, Player::Yellow);
        assert_eq!(board.distance_to_goal(Player::Red, 0), 27);
        assert_eq!(board.distance_to_goal(Player::Red, 1), 26);
        assert_eq!(board.distance_to_goal(Player::Red, 27), 0);
        assert_eq!(board.distance_to_goal(Player::Red, 26), 1);
    }

    #[test]
    fn clockwise_distance_1() {
        let board = Board::new(Player::Red, Player::Yellow);
        assert_eq!(board.clockwise_distance(0, 1), 1);
        assert_eq!(board.clockwise_distance(0, 10), 10);
        assert_eq!(board.clockwise_distance(26, 27), 1);
        assert_eq!(board.clockwise_distance(27, 0), 1);
        assert_eq!(board.clockwise_distance(3, 0), 25);
    }
}

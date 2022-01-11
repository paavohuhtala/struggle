use smallvec::{self, SmallVec};

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

#[derive(Clone)]
pub struct Board {
    pub tiles: [BoardCell; 7 * 4],
    pub goals: [Goal; 4],
    pub home_bases: [HomeBase; 4],
}

pub type MoveVec = SmallVec<[ValidMove; 4]>;
pub type PieceVec = SmallVec<[PiecePosition; 4]>;

impl Board {
    pub fn new() -> Self {
        Board {
            tiles: [None; 7 * 4],
            goals: COLORS.map(|_| [None; 4]).try_into().unwrap(),
            home_bases: COLORS.map(|_| HomeBase::new()),
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
                return Some(g[0].unwrap());
            } else {
                None
            }
        })
    }

    pub fn get_pieces(&self, player: Player, enemy: Player) -> (PieceVec, PieceVec) {
        let mut player_positions = PieceVec::with_capacity(4);
        let mut enemy_positions = PieceVec::with_capacity(4);

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
            match piece {
                Some(_) => player_positions.push(PiecePosition::Goal(i as u8)),
                _ => {}
            }
        }

        let enemy_goal = &self.goals[enemy as usize];

        for (i, piece) in enemy_goal.iter().enumerate() {
            match piece {
                Some(_) => enemy_positions.push(PiecePosition::Goal(i as u8)),
                _ => {}
            }
        }

        (player_positions, enemy_positions)
    }

    pub fn get_moves(&self, dice: u8, player: Player, enemy: Player) -> MoveVec {
        let mut moves = MoveVec::with_capacity(4);

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
                        Some(pos) => match goal.get(pos as usize) {
                            Some(None) => {
                                moves.push(ValidMove::MoveToGoal {
                                    from_board: current_pos,
                                    to_goal: pos,
                                });
                            }
                            _ => {}
                        },
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

                    match goal.get(new_pos as usize) {
                        Some(None) => {
                            moves.push(ValidMove::MoveInGoal {
                                from_goal: i,
                                to_goal: new_pos,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        if moves.len() == 0 {
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
    }

    pub fn clockwise_distance(&self, from: u8, to: u8) -> u8 {
        let mut distance = 0;
        let mut current = from;

        while current != to {
            current = (current + 1) % self.tiles.len() as u8;
            distance += 1;
        }

        distance
    }

    pub fn distance_to_goal(&self, player: Player, pos: u8) -> u8 {
        match player {
            Player::Red => 27 - pos,
            _ => {
                let start = Self::get_start(player) - 1;

                if pos > start {
                    28 - pos + start
                } else {
                    start - pos
                }
            }
        }
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
        let mut board = Board::new();
        board.tiles[27] = Some(Player::Red);
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
        let mut board = Board::new();
        board.tiles[26] = Some(Player::Red);
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
        let mut board = Board::new();
        board.tiles[27] = Some(Player::Yellow);
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
        let board = Board::new();
        assert_eq!(board.distance_to_goal(Player::Yellow, 0), 13);
        assert_eq!(board.distance_to_goal(Player::Yellow, 27), 14);
        assert_eq!(board.distance_to_goal(Player::Yellow, 13), 0);
        assert_eq!(board.distance_to_goal(Player::Yellow, 14), 27);
    }

    #[test]
    fn red_distance_to_goal() {
        let board = Board::new();
        assert_eq!(board.distance_to_goal(Player::Red, 0), 27);
        assert_eq!(board.distance_to_goal(Player::Red, 1), 26);
        assert_eq!(board.distance_to_goal(Player::Red, 27), 0);
        assert_eq!(board.distance_to_goal(Player::Red, 26), 1);
    }

    #[test]
    fn clockwise_distance_1() {
        let board = Board::new();
        assert_eq!(board.clockwise_distance(0, 1), 1);
        assert_eq!(board.clockwise_distance(0, 10), 10);
        assert_eq!(board.clockwise_distance(26, 27), 1);
        assert_eq!(board.clockwise_distance(27, 0), 1);
        assert_eq!(board.clockwise_distance(3, 0), 25);
    }
}

use std::borrow::Cow;

use ::rand::{prelude::*, rngs::SmallRng};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::struggle::{Board, PiecePosition, Player, ValidMove};

pub trait StrugglePlayer: Clone {
    fn name(&self) -> Cow<'static, str>;

    fn select_move<'a>(
        &mut self,
        ctx: &'a GameContext,
        board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove;
}

pub struct GameContext {
    pub current_player: Player,
    pub other_player: Player,
    pub dice: u8,
}

// Randomly selects any legal move
#[derive(Clone)]
pub struct RandomPlayer;

impl StrugglePlayer for RandomPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &'a GameContext,
        _board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        moves.choose(rng).unwrap()
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from("Random")
    }
}

// Eat whenever possible, otherwise select move randomly
#[derive(Clone)]
pub struct RandomEaterPlayer;

impl StrugglePlayer for RandomEaterPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &'a GameContext,
        _board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let eating_moves = moves.iter().find(|mov| mov.eats());

        match eating_moves {
            Some(mov) => mov,
            None => moves.choose(rng).unwrap(),
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from("RandomEater")
    }
}

// Avoid eating at all costs
#[derive(Clone)]
pub struct RandomDietPlayer;

impl StrugglePlayer for RandomDietPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &'a GameContext,
        _board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let diet_compatible = moves.iter().filter(|mov| !mov.eats()).collect_vec();

        match diet_compatible.len() {
            0 => moves.choose(rng).unwrap(),
            _ => diet_compatible.choose(rng).unwrap(),
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from("RandomDiet")
    }
}

// Prioritise moving pieces over introducing new ones.
// It doesn't matter who moves, as long as someone moves.
#[derive(Clone)]
pub struct MoveItPlayer;

impl StrugglePlayer for MoveItPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &'a GameContext,
        _board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let moving_moves = moves
            .iter()
            .filter(|mov| match mov {
                ValidMove::AddNewPiece { .. } => false,
                _ => true,
            })
            .collect_vec();

        match moving_moves.len() {
            0 => moves.choose(rng).unwrap(),
            _ => moving_moves.choose(rng).unwrap(),
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from("MoveIt")
    }
}

// Focus on getting everyone on the board, then play randomly
#[derive(Clone)]
pub struct ParticipationAwardPlayer;

impl StrugglePlayer for ParticipationAwardPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &'a GameContext,
        _board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let participatory_moves = moves
            .iter()
            .filter(|mov| match mov {
                ValidMove::AddNewPiece { .. } => true,
                _ => false,
            })
            .collect_vec();

        match participatory_moves.len() {
            0 => moves.choose(rng).unwrap(),
            _ => participatory_moves.choose(rng).unwrap(),
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from("ParticipationAward")
    }
}

pub type HeuristicFunction = fn(board: &Board, player: Player, enemy: Player) -> f64;

#[derive(Clone)]
pub struct GameTreePlayer<F>
where
    F: Fn(&Board, Player, Player) -> f64,
{
    pub heuristic: F,
    pub max_depth: u8,

    name: &'static str,
}

impl<F: Fn(&Board, Player, Player) -> f64> GameTreePlayer<F> {
    pub fn new(f: F, max_depth: u8, name: &'static str) -> Self {
        GameTreePlayer {
            heuristic: f,
            max_depth,
            name,
        }
    }

    fn expectimax(
        &self,
        board: &Board,
        maximizing_player: Player,
        minimizing_player: Player,
        maxiziming: bool,
        depth: u8,
    ) -> f64 {
        if depth == 0 {
            return (self.heuristic)(board, maximizing_player, minimizing_player);
        }

        if maxiziming {
            let mut total_value = 0.0;

            for dice_roll in 1..=6 {
                let moves = board.get_moves(dice_roll, maximizing_player, minimizing_player);

                let mut best_score = std::f64::NEG_INFINITY;

                for mov in &moves {
                    let mut new_board = board.clone();
                    new_board.perform_move(maximizing_player, mov);

                    let score = if dice_roll == 6 {
                        self.expectimax(
                            &new_board,
                            maximizing_player,
                            minimizing_player,
                            true,
                            depth - 1,
                        )
                    } else {
                        self.expectimax(
                            &new_board,
                            maximizing_player,
                            minimizing_player,
                            false,
                            depth - 1,
                        )
                    };

                    best_score = best_score.max(score);
                }

                total_value += best_score;
            }

            total_value / 6.0
        } else {
            let mut total_value = 0.0;

            for dice_roll in 1..=6 {
                let moves = board.get_moves(dice_roll, minimizing_player, maximizing_player);

                let mut min_score = std::f64::INFINITY;

                for mov in &moves {
                    let mut new_board = board.clone();
                    new_board.perform_move(minimizing_player, mov);

                    let score = if dice_roll == 6 {
                        self.expectimax(
                            &new_board,
                            maximizing_player,
                            minimizing_player,
                            false,
                            depth - 1,
                        )
                    } else {
                        self.expectimax(
                            &new_board,
                            maximizing_player,
                            minimizing_player,
                            true,
                            depth - 1,
                        )
                    };

                    min_score = min_score.min(score);
                }

                total_value += min_score;
            }

            total_value / 6.0
        }
    }
}

impl<F: Fn(&Board, Player, Player) -> f64 + Clone> StrugglePlayer for GameTreePlayer<F> {
    fn select_move<'a>(
        &mut self,
        ctx: &'a GameContext,
        board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let mut scored_moves = moves
            .iter()
            .map(|mov| {
                let mut new_board = board.clone();
                new_board.perform_move(ctx.current_player, mov);

                let score = self.expectimax(
                    &new_board,
                    ctx.current_player,
                    ctx.other_player,
                    false,
                    self.max_depth,
                );

                (mov, score)
            })
            .collect::<SmallVec<[(&ValidMove, f64); 4]>>();

        let tied = scored_moves
            .iter()
            .all(|(_, score)| score == &scored_moves[0].1);

        if tied {
            return moves.choose(rng).unwrap();
        } else {
            scored_moves.sort_by(|(_, score1), (_, score2)| score2.partial_cmp(score1).unwrap());
            let best = scored_moves[0].0;

            return best;
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from(format!("{}({})", self.name, self.max_depth))
    }
}

pub fn default_heuristic(board: &Board, player: Player, enemy: Player) -> f64 {
    let mut score = 0.0;

    match board.get_winner() {
        Some(winner) if winner == player => {
            return 10000000.0;
        }
        Some(_) => {
            return -10000000.0;
        }
        None => {}
    }

    let (own_pieces, enemy_pieces) = board.get_pieces(player, enemy);

    let my_home = Board::get_start(player);
    let enemy_home = Board::get_start(enemy);

    for piece in &own_pieces {
        match piece {
            PiecePosition::Board(i) => {
                let distance_to_goal = board.distance_to_goal(player, *i);

                // discourage moving to enemy home
                if *i == enemy_home {
                    score += 50.0;
                } else {
                    if distance_to_goal <= 2 {
                        score += 200.0;
                    } else {
                        score += 100.0;
                    }
                }

                for enemy_i in enemy_pieces
                    .iter()
                    .copied()
                    .filter_map(PiecePosition::as_board_index)
                {
                    let distance_to_enemy = board.clockwise_distance(*i, enemy_i);

                    // Small bonus for being within eating distance
                    if distance_to_enemy >= 1 && distance_to_enemy <= 6 {
                        score += 20.0;
                    }
                }
            }
            PiecePosition::Goal(_) => {
                score += 10000.0;
            }
        }
    }

    for piece in enemy_pieces {
        match piece {
            PiecePosition::Board(i) => {
                if i == my_home {
                    score -= 150.0;
                } else {
                    score -= 300.0;
                }

                for own_i in own_pieces
                    .iter()
                    .copied()
                    .filter_map(PiecePosition::as_board_index)
                {
                    let distance_to_own = board.clockwise_distance(i, own_i);

                    // Penalty for being within eating distance
                    if distance_to_own >= 1 && distance_to_own <= 6 {
                        score -= 20.0;
                    }
                }
            }
            PiecePosition::Goal(_) => {
                score -= 15000.0;
            }
        }
    }

    score
}

pub fn expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: default_heuristic,
        max_depth: depth,
        name: "Expectimax",
    }
}

pub fn confused_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |b, p1, p2| default_heuristic(b, p2, p1),
        max_depth: depth,
        name: "ConfusedExpectimax",
    }
}

pub fn worst_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |b, p1, p2| -default_heuristic(b, p1, p2),
        max_depth: depth,
        name: "WorstExpectimax",
    }
}

pub fn random_expectimax() -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |_, _, _| rand::thread_rng().gen(),
        max_depth: 0,
        name: "RandomExpectimax",
    }
}

pub fn participatory_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| 4.0 - board.home_bases[player as usize].pieces_waiting as f64,
        max_depth: depth,
        name: "ParticipatoryExpectimax",
    }
}

pub fn one_at_a_time_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| board.home_bases[player as usize].pieces_waiting as f64,
        max_depth: depth,
        name: "OneAtATimeExpectimax",
    }
}

fn count_moves_heuristic(board: &Board, player: Player, enemy: Player) -> f64 {
    (1..=6)
        .map(|die| board.get_moves(die, player, enemy).len() as f64)
        .sum::<f64>()
        / 6.0
}

pub fn maximize_options_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: count_moves_heuristic,
        max_depth: depth,
        name: "MaximizeOptionsExpectimax",
    }
}

pub fn minimize_options_expectimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, enemy| -count_moves_heuristic(board, enemy, player),
        max_depth: depth,
        name: "MinimizeOptionsExpectimax",
    }
}

#[derive(Clone)]
pub struct DilutedPlayer<P: StrugglePlayer>(pub P, pub f64);

impl<P: StrugglePlayer> StrugglePlayer for DilutedPlayer<P> {
    fn select_move<'a>(
        &mut self,
        ctx: &'a GameContext,
        board: &'a Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        if rng.gen::<f64>() < self.1 {
            self.0.select_move(ctx, board, moves, rng)
        } else {
            moves.choose(rng).unwrap()
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from(format!("{} {:.0}%", self.0.name(), self.1 * 100.0))
    }
}

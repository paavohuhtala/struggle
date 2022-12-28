use std::borrow::Cow;

use ::rand::{prelude::*, rngs::SmallRng};
use arrayvec::ArrayVec;
use itertools::Itertools;

use super::{
    board::{Board, PiecePosition, ValidMove},
    PlayerColor,
};

pub trait StrugglePlayer: Clone + Send + Sync {
    fn name(&self) -> Cow<'static, str>;

    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove;
}

pub struct GameContext {
    pub current_player: PlayerColor,
    pub other_player: PlayerColor,
    pub dice: u8,
}

// Randomly selects any legal move
#[derive(Clone)]
pub struct RandomPlayer;

impl StrugglePlayer for RandomPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &Board,
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
        _ctx: &GameContext,
        _board: &Board,
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
        _ctx: &GameContext,
        _board: &Board,
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

pub type HeuristicFunction = fn(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64;

#[derive(Clone)]
pub struct GameTreePlayer<F>
where
    F: Fn(&Board, PlayerColor, PlayerColor) -> f64,
{
    pub heuristic: F,
    pub max_depth: u8,

    name: &'static str,
}

impl<F: Fn(&Board, PlayerColor, PlayerColor) -> f64> GameTreePlayer<F> {
    pub fn new(f: F, max_depth: u8, name: &'static str) -> Self {
        GameTreePlayer {
            heuristic: f,
            max_depth,
            name,
        }
    }

    fn expectiminimax(
        &self,
        board: &Board,
        maximizing_player: PlayerColor,
        minimizing_player: PlayerColor,
        maxiziming: bool,
        max_depth: u8,
        depth: u8,
    ) -> f64 {
        if depth == max_depth {
            return (self.heuristic)(board, maximizing_player, minimizing_player);
        }

        if maxiziming {
            let mut expected_value = 0.0;

            for dice_roll in 1..=6 {
                let moves = board.get_moves(dice_roll, maximizing_player, minimizing_player);

                let mut max_score = std::f64::NEG_INFINITY;

                for mov in &moves {
                    let new_board = board.with_move(maximizing_player, mov);

                    let score = self.expectiminimax(
                        &new_board,
                        maximizing_player,
                        minimizing_player,
                        // this should take 6 into account, but that made things worse
                        false,
                        max_depth,
                        depth + 1,
                    );

                    max_score = max_score.max(score);
                }

                expected_value += max_score / 6.0;
            }

            expected_value
        } else {
            let mut expected_value = 0.0;

            for dice_roll in 1..=6 {
                let moves = board.get_moves(dice_roll, minimizing_player, maximizing_player);

                let mut min_score = std::f64::INFINITY;

                for mov in &moves {
                    let new_board = board.with_move(minimizing_player, mov);

                    let score = self.expectiminimax(
                        &new_board,
                        maximizing_player,
                        minimizing_player,
                        // this should take 6 into account, but that made things worse
                        true,
                        max_depth,
                        depth + 1,
                    );

                    min_score = min_score.min(score);
                }

                expected_value += min_score / 6.0;
            }

            expected_value
        }
    }
}

impl<F: Fn(&Board, PlayerColor, PlayerColor) -> f64 + Clone + Send + Sync> StrugglePlayer
    for GameTreePlayer<F>
{
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        let scored_moves = moves
            .iter()
            .map(|mov| {
                let new_board = board.with_move(ctx.current_player, mov);

                let score = self.expectiminimax(
                    &new_board,
                    ctx.current_player,
                    ctx.other_player,
                    // This should technically be ctx.dice == 6,
                    // but for some reason that is making the AI perform worse :(
                    false,
                    self.max_depth,
                    0,
                );

                (mov, score)
            })
            .collect::<ArrayVec<(&ValidMove, f64), 4>>();

        let tied = scored_moves
            .iter()
            .all(|(_, score)| score == &scored_moves[0].1);

        if tied {
            return moves.choose(rng).unwrap();
        } else {
            scored_moves
                .iter()
                .max_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap())
                .unwrap()
                .0
        }
    }

    fn name(&self) -> Cow<'static, str> {
        Cow::from(format!("{}({})", self.name, self.max_depth))
    }
}

pub fn default_heuristic(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
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

    for piece in own_pieces {
        match piece {
            PiecePosition::Board(i) => {
                let distance_to_goal = board.distance_to_goal(player, *i);
                let relative_distance = 1.0 - distance_to_goal as f64 / 28.0;

                // discourage moving to enemy home
                if *i == enemy_home {
                    score += 50.0;
                } else if distance_to_goal <= 1 {
                    score += 175.0;
                } else {
                    score += 100.0 + relative_distance;
                }

                for enemy_i in enemy_pieces
                    .iter()
                    .copied()
                    .filter_map(PiecePosition::as_board_index)
                {
                    let distance_to_enemy = board.clockwise_distance(*i, enemy_i);

                    // Small bonus for being within eating distance
                    if (1..=6).contains(&distance_to_enemy) {
                        score += 40.0;
                    }
                }
            }
            PiecePosition::Goal(n) => {
                score += 10000.0 + (*n as f64 / 3.0) * 10.0;
            }
        }
    }

    for piece in enemy_pieces {
        match piece {
            PiecePosition::Board(i) => {
                if *i == my_home {
                    score -= 150.0;
                } else {
                    score -= 300.0;
                }

                for own_i in own_pieces
                    .iter()
                    .copied()
                    .filter_map(PiecePosition::as_board_index)
                {
                    let distance_to_own = board.clockwise_distance(*i, own_i);

                    // Penalty for being within eating distance
                    if (1..=6).contains(&distance_to_own) {
                        score -= 25.0;
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

pub fn expectiminimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: default_heuristic,
        max_depth: depth,
        name: "Expectiminimax",
    }
}

pub fn worst_expectiminimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |b, p1, p2| -default_heuristic(b, p1, p2),
        max_depth: depth,
        name: "WorstExpectiminimax",
    }
}

pub fn participation_award(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| -(board.home_bases[player as usize].pieces_waiting as f64),
        max_depth: depth,
        name: "ParticipationAward",
    }
}

pub fn one_at_a_time(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| board.home_bases[player as usize].pieces_waiting as f64,
        max_depth: depth,
        name: "OneAtATime",
    }
}

fn one_at_a_time_heuristic(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
    let (own_pieces, enemy_pieces) = board.get_pieces(player, enemy);

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

    let mut own_pieces_on_board = 0;

    for piece in own_pieces {
        match piece {
            PiecePosition::Board(_) => {
                own_pieces_on_board += 1;
            }
            PiecePosition::Goal(_) => {
                score += 10000.0;
            }
        }
    }

    if own_pieces_on_board > 1 {
        score -= (own_pieces_on_board - 1) as f64 * 100.0;
    }

    for piece in enemy_pieces {
        match piece {
            PiecePosition::Board(_) => {
                score -= 2000.0;
            }
            PiecePosition::Goal(_) => {
                score -= 10000.0;
            }
        }
    }

    score
}

pub fn one_at_a_time_deluxe(max_depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: one_at_a_time_heuristic,
        max_depth,
        name: "OneAtATimeDeluxe",
    }
}

fn count_moves_heuristic(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
    (1..=6)
        .map(|die| board.get_moves(die, player, enemy).len() as f64)
        .sum::<f64>()
        / 6.0
}

pub fn maximize_options(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: count_moves_heuristic,
        max_depth: depth,
        name: "MaximizeOptions",
    }
}

pub fn minimize_options(max_depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, enemy| -count_moves_heuristic(board, enemy, player),
        max_depth,
        name: "MinimizeOptions",
    }
}

fn maximize_length_heuristic(board: &Board) -> f64 {
    if board.get_winner().is_some() {
        return -1000000.0;
    }

    let players = board.players();
    let (a_pieces, b_pieces) = board.get_pieces(players.0, players.1);
    let mut score = 0.0;

    score -= board.home_bases[players.0 as usize].pieces_waiting as f64 * 2.0;
    score -= board.home_bases[players.1 as usize].pieces_waiting as f64 * 2.0;

    for (player, pieces) in &[(players.0, a_pieces), (players.1, b_pieces)] {
        for piece in pieces.iter() {
            match piece {
                PiecePosition::Board(pos) => {
                    let distance_to_goal = board.distance_to_goal(*player, *pos);
                    let relative_distance = 1.0 - distance_to_goal as f64 / 28.0;

                    score -= relative_distance * 50.0;
                }
                PiecePosition::Goal(_) => {
                    score -= 1000.0;
                }
            }
        }
    }

    score
}

pub fn maximize_length_expectiminimax(max_depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, _player, _enemy| maximize_length_heuristic(board),
        max_depth,
        name: "MaximizeLength",
    }
}

#[derive(Clone)]
pub struct StatefulGetItOverWith {
    supporting: Option<PlayerColor>,
    max_depth: u8,
}

pub fn stateful_get_it_over_with(max_depth: u8) -> impl StrugglePlayer {
    StatefulGetItOverWith {
        supporting: None,
        max_depth,
    }
}

impl StrugglePlayer for StatefulGetItOverWith {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("GetItOverWith")
    }

    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [ValidMove],
        rng: &mut SmallRng,
    ) -> &'a ValidMove {
        if self.supporting.is_none() {
            let own_pieces_in_goal = board.pieces_in_goal(ctx.current_player);
            let enemy_pieces_in_goal = board.pieces_in_goal(ctx.other_player);

            self.supporting = if own_pieces_in_goal >= 1 {
                Some(ctx.current_player)
            } else if enemy_pieces_in_goal >= 1 {
                Some(ctx.other_player)
            } else {
                None
            };
        }

        if let Some(supporting) = self.supporting {
            GameTreePlayer {
                max_depth: self.max_depth,
                name: "GetItOverWithInternal",
                heuristic: |board, player, enemy| {
                    if player == supporting {
                        default_heuristic(board, player, enemy)
                    } else {
                        default_heuristic(board, enemy, player)
                    }
                },
            }
            .select_move(&ctx, board, moves, rng)
        } else {
            RandomPlayer.select_move(ctx, board, moves, rng)
        }
    }
}

#[derive(Clone)]
pub struct DilutedPlayer<P: StrugglePlayer>(pub P, pub f64);

impl<P: StrugglePlayer> StrugglePlayer for DilutedPlayer<P> {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
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

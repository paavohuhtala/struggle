use std::{borrow::Cow, sync::Arc};

use ::rand::{prelude::*, rngs::SmallRng};
use itertools::Itertools;
use ordered_float::OrderedFloat;

use crate::game::NamedPlayer;

use super::{
    board::{Board, PiecePosition, StruggleMove},
    transposition_table::{get_board_hash, TranspositionTable},
    PlayerColor,
};

pub trait StrugglePlayer: Clone + Send + Sync + NamedPlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove;

    fn reset(&mut self) {}

    fn total_evaluations(&self) -> u64 {
        0
    }
}

pub struct GameContext {
    pub current_player: PlayerColor,
    pub other_player: PlayerColor,
    pub dice: u8,
}

// Randomly selects any legal move
#[derive(Clone)]
pub struct RandomPlayer;

impl NamedPlayer for RandomPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Random")
    }
}

impl StrugglePlayer for RandomPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        moves.choose(rng).unwrap()
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
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        let eating_moves = moves.iter().find(|mov| mov.eats());

        match eating_moves {
            Some(mov) => mov,
            None => moves.choose(rng).unwrap(),
        }
    }
}

impl NamedPlayer for RandomEaterPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("RandomEater")
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
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        let diet_compatible = moves.iter().filter(|mov| !mov.eats()).collect_vec();

        match diet_compatible.len() {
            0 => moves.choose(rng).unwrap(),
            _ => diet_compatible.choose(rng).unwrap(),
        }
    }
}

impl NamedPlayer for RandomDietPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("RandomDiet")
    }
}

fn score_move(rng: &mut SmallRng, mov: &StruggleMove) -> OrderedFloat<f64> {
    let score = match mov {
        StruggleMove::AddNewPiece { eats } => {
            if *eats {
                150.0
            } else {
                50.0
            }
        }
        StruggleMove::MovePiece {
            from: _,
            to: _,
            eats,
        } => {
            if *eats {
                100.0
            } else {
                1.0
            }
        }
        StruggleMove::MoveToGoal {
            from_board: _,
            to_goal: _,
        } => 10.0,
        StruggleMove::MoveInGoal {
            from_goal: _,
            to_goal: _,
        } => 1.0,
        StruggleMove::SkipTurn => 0.0,
    };
    OrderedFloat(score + rng.gen::<f64>())
}

// Selects the best move using a simple heuristic
// The board state is not inspected, only the move type
#[derive(Clone)]
pub struct ScoreMovePlayer;

impl StrugglePlayer for ScoreMovePlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        moves.iter().max_by_key(|mov| score_move(rng, mov)).unwrap()
    }
}

impl NamedPlayer for ScoreMovePlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("ScoreMove")
    }
}

// Selects the worst move using the same heuristic as ScoreMovePlayer, but negated
#[derive(Clone)]
pub struct WorstScoreMovePlayer;

impl StrugglePlayer for WorstScoreMovePlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        moves.iter().min_by_key(|mov| score_move(rng, mov)).unwrap()
    }
}

impl NamedPlayer for WorstScoreMovePlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("WorstScoreMove")
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

    pub evaluations: u64,

    cache: Arc<TranspositionTable>,
}

const INFO_LOGGING: bool = false;
const VERBOSE_LOGGING: bool = false;
const USE_TRANSPOSITION_TABLE: bool = false;
const USE_TRANSPOSITION_TABLE_FOR_NON_LEAFS: bool = false;

const WIN_SCORE: f64 = 1e10;

impl<F: Fn(&Board, PlayerColor, PlayerColor) -> f64> GameTreePlayer<F> {
    pub fn new(f: F, max_depth: u8, name: &'static str) -> Self {
        GameTreePlayer {
            heuristic: f,
            max_depth,
            name,
            evaluations: 0,
            cache: Default::default(),
        }
    }

    fn expectiminimax(
        &mut self,
        board: &Board,
        current_player: PlayerColor,
        maximizing_player: PlayerColor,
        minimizing_player: PlayerColor,
        max_depth: u8,
        depth: u8,
        // Alpha: minimum guaranteed score for the maximizing player
        alpha: f64,
        // Beta: maximum guaranteed score for the minimizing player
        beta: f64,
        rng: &mut SmallRng,
    ) -> f64 {
        let hash = get_board_hash(board, current_player);

        if USE_TRANSPOSITION_TABLE {
            if let Some(value) = self.cache.get(hash, depth) {
                return value as f64;
            }
        }

        self.evaluations += 1;

        if depth == max_depth {
            let value = (self.heuristic)(board, maximizing_player, minimizing_player);

            if USE_TRANSPOSITION_TABLE {
                self.cache.insert_if_better(hash, value as f32, depth);
            }

            return value;
        }

        let mut expected_value = 0.0;

        for dice_roll in 1..=6 {
            let mut alpha = alpha;
            let mut beta = beta;

            let multiplier = match dice_roll {
                6 => 1.0 / 6.0,
                _ => 1.0,
            };

            let score = if current_player == maximizing_player {
                let mut moves: arrayvec::ArrayVec<StruggleMove, 4> =
                    board.get_moves(dice_roll, maximizing_player, minimizing_player);
                moves.sort_by_key(|mov| OrderedFloat(-score_move(rng, mov)));

                let mut max_score = f64::NEG_INFINITY;
                let mut best_move = moves.first().unwrap();

                for mov in &moves {
                    let board = board.with_move(maximizing_player, mov);

                    let (score, guaranteed_win) = match board.get_winner() {
                        Some(player) if player == maximizing_player => (WIN_SCORE, true),
                        Some(_) => {
                            panic!("This should never happen: minimizing player won after maximizing player's move")
                        }
                        None => (
                            self.expectiminimax(
                                &board,
                                if dice_roll == 6 {
                                    maximizing_player
                                } else {
                                    minimizing_player
                                },
                                maximizing_player,
                                minimizing_player,
                                max_depth,
                                depth + 1,
                                alpha,
                                beta,
                                rng,
                            ),
                            false,
                        ),
                    };

                    if score > max_score {
                        best_move = mov;
                    }

                    max_score = max_score.max(score);
                    alpha = alpha.max(score);

                    // The maximizing can guarantee a win with this move, no need to look further
                    if guaranteed_win {
                        break;
                    }

                    // Alpha-beta pruning: minimizing player will never allow this move
                    if max_score >= beta {
                        break;
                    }
                }

                if VERBOSE_LOGGING {
                    println!(
                        "At depth {}, maximizing player chose move {:?}",
                        depth, best_move
                    );
                }

                max_score
            } else {
                let mut moves = board.get_moves(dice_roll, minimizing_player, maximizing_player);
                moves.sort_by_key(|mov| OrderedFloat(-score_move(rng, mov)));

                let mut min_score = f64::INFINITY;

                for mov in &moves {
                    let board = board.with_move(minimizing_player, mov);

                    let (score, guaranteed_loss) = match board.get_winner() {
                        Some(player) if player == minimizing_player => (-WIN_SCORE, true),
                        Some(_) => {
                            panic!("This should never happen: maximizing player won after minimizing player's move")
                        }
                        None => (
                            self.expectiminimax(
                                &board,
                                if dice_roll == 6 {
                                    minimizing_player
                                } else {
                                    maximizing_player
                                },
                                maximizing_player,
                                minimizing_player,
                                max_depth,
                                depth + 1,
                                alpha,
                                beta,
                                rng,
                            ),
                            false,
                        ),
                    };

                    min_score = min_score.min(score);
                    beta = beta.min(score);

                    // The minimizing player can guarantee a loss with this move, no need to look further
                    if guaranteed_loss {
                        break;
                    }

                    // Alpha-beta pruning: maximizing player will never allow this move
                    if min_score <= alpha {
                        break;
                    }
                }

                min_score
            };

            expected_value += score * multiplier;
        }

        expected_value /= 6.0;

        if USE_TRANSPOSITION_TABLE && USE_TRANSPOSITION_TABLE_FOR_NON_LEAFS {
            self.cache
                .insert_if_better(hash, expected_value as f32, depth);
        }

        expected_value
    }
}

impl<F: Fn(&Board, PlayerColor, PlayerColor) -> f64 + Clone + Send + Sync> StrugglePlayer
    for GameTreePlayer<F>
{
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        if moves.len() == 1 {
            return moves.first().unwrap();
        }

        if VERBOSE_LOGGING {
            println!("{} is selecting a move...", self.name());
        }

        moves
            .iter()
            .max_by_key(|mov| {
                let new_board = board.with_move(ctx.current_player, mov);

                let next_turn = match ctx.dice {
                    6 => ctx.current_player,
                    _ => ctx.other_player,
                };

                let score = self.expectiminimax(
                    &new_board,
                    next_turn,
                    ctx.current_player,
                    ctx.other_player,
                    self.max_depth,
                    0,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    rng,
                );

                if INFO_LOGGING {
                    println!("Move {:?} scored: {}", mov, score);
                }

                // Add a bit of random noise to break ties
                OrderedFloat(score + rng.gen::<f64>())
            })
            .unwrap()
    }

    fn reset(&mut self) {
        self.evaluations = 0;
    }

    fn total_evaluations(&self) -> u64 {
        self.evaluations
    }
}

impl<F: Fn(&Board, PlayerColor, PlayerColor) -> f64> NamedPlayer for GameTreePlayer<F> {
    fn name(&self) -> Cow<'static, str> {
        Cow::from(format!("{}({})", self.name, self.max_depth))
    }
}

fn heuristic_evaluate_side(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
    let (own_pieces, enemy_pieces) = board.get_pieces(player, enemy);

    let my_home = Board::get_start(player);
    let enemy_home = Board::get_start(enemy);
    let my_pieces_waiting = board.home_bases[player as usize].pieces_waiting;
    let enemy_pieces_waiting = board.home_bases[enemy as usize].pieces_waiting;

    const BASE_PIECE_SCORE: f64 = 550.0;
    const ENEMY_HOME_PENALTY: f64 = 100.0;
    const OWN_HOME_PENALTY: f64 = 100.0;
    const ADVANCE_PIECE_MULTIPLIER: f64 = 200.0;
    const AT_EATING_DISTANCE_BONUS: f64 = 100.0;
    const BASE_PIECE_IN_GOAL_SCORE: f64 = 1000.0;
    const ADVANCE_PIECE_IN_GOAL_MULTIPLIER: f64 = 10.0;
    const RELATIVE_ADVANCEMENT_POWER: f64 = 1.1;
    const CAN_ENTER_GOAL_BONUS: f64 = 20.0;

    let mut score = 0.0;

    for piece in own_pieces {
        match piece {
            PiecePosition::Board(i) => {
                score += BASE_PIECE_SCORE;

                let distance_to_goal = board.distance_to_goal_entrance(player, *i);
                let relative_advancement = 1.0 - distance_to_goal as f64 / 28.0;

                // Encourage moving pieces that are already close to the goal further
                score += relative_advancement.powf(RELATIVE_ADVANCEMENT_POWER)
                    * ADVANCE_PIECE_MULTIPLIER;

                // Penalize for being in the enemy home, because it's risky (unless there are no pieces waiting)
                if *i == enemy_home && enemy_pieces_waiting > 0 {
                    score -= ENEMY_HOME_PENALTY;
                }

                // Give a small penalty for being in your own home (if there are still pieces waiting)
                // because it blocks mobilizing other pieces
                if *i == my_home && my_pieces_waiting > 0 {
                    score -= OWN_HOME_PENALTY;
                }

                for enemy_i in enemy_pieces
                    .iter()
                    .copied()
                    .filter_map(PiecePosition::as_board_index)
                {
                    let distance_to_enemy = board.clockwise_distance(*i, enemy_i);

                    // Small bonus for being within eating distance
                    if distance_to_enemy >= 1 && distance_to_enemy <= 6 {
                        score += AT_EATING_DISTANCE_BONUS;
                    }
                }

                // If the piece can enter a goal slot with a roll of 1-6, give a bonus for each slot
                for goal_position in 0..4u8 {
                    let is_free = board.goals[player as usize][goal_position as usize].is_none();
                    if !is_free {
                        continue;
                    }
                    let distance = board.distance_to_goal_slot(player, *i, goal_position);
                    if distance >= 1 && distance <= 6 {
                        score += CAN_ENTER_GOAL_BONUS;
                    }
                }
            }
            PiecePosition::Goal(n) => {
                score +=
                    BASE_PIECE_IN_GOAL_SCORE + (*n as f64 / 3.0) * ADVANCE_PIECE_IN_GOAL_MULTIPLIER;
            }
        }
    }

    score
}

pub fn default_heuristic(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
    match board.get_winner() {
        Some(winner) if winner == player => {
            return WIN_SCORE;
        }
        Some(_) => {
            return -WIN_SCORE;
        }
        None => {}
    }

    let my_score = heuristic_evaluate_side(board, player, enemy);
    let enemy_score = heuristic_evaluate_side(board, enemy, player);
    my_score - enemy_score
}

pub fn minimal_heuristic(board: &Board, player: PlayerColor, enemy: PlayerColor) -> f64 {
    match board.get_winner() {
        Some(winner) if winner == player => {
            return WIN_SCORE;
        }
        Some(_) => {
            return -WIN_SCORE;
        }
        None => {}
    }

    let mut score = 0.0;

    let (my_pieces, enemy_pieces) = board.get_pieces(player, enemy);

    for piece in my_pieces {
        match piece {
            PiecePosition::Board(_) => {
                score += 1.0;
            }
            PiecePosition::Goal(_) => {
                score += 10.0;
            }
        }
    }

    for piece in enemy_pieces {
        match piece {
            PiecePosition::Board(_) => {
                score -= 10.0;
            }
            PiecePosition::Goal(_) => {
                score -= 100.0;
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
        evaluations: 0,
        cache: Default::default(),
    }
}

pub fn expectiminimax_mvp(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: minimal_heuristic,
        max_depth: depth,
        name: "ExpectiminimaxBasic",
        evaluations: 0,
        cache: Default::default(),
    }
}

pub fn worst_expectiminimax(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |b, p1, p2| -default_heuristic(b, p1, p2),
        max_depth: depth,
        name: "WorstExpectiminimax",
        evaluations: 0,
        cache: Default::default(),
    }
}

pub fn participation_trophy(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| -(board.home_bases[player as usize].pieces_waiting as f64),
        max_depth: depth,
        name: "ParticipationTrophy",
        evaluations: 0,
        cache: Default::default(),
    }
}

pub fn one_at_a_time(depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, _| board.home_bases[player as usize].pieces_waiting as f64,
        max_depth: depth,
        name: "OneAtATime",
        evaluations: 0,
        cache: Default::default(),
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
        evaluations: 0,
        cache: Default::default(),
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
        evaluations: 0,
        cache: Default::default(),
    }
}

pub fn minimize_options(max_depth: u8) -> impl StrugglePlayer {
    GameTreePlayer {
        heuristic: |board, player, enemy| -count_moves_heuristic(board, enemy, player),
        max_depth,
        name: "MinimizeOptions",
        evaluations: 0,
        cache: Default::default(),
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
                    let distance_to_goal = board.distance_to_goal_entrance(*player, *pos);
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
        evaluations: 0,
        cache: Default::default(),
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
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
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
                evaluations: 0,
                cache: Default::default(),
            }
            .select_move(&ctx, board, moves, rng)
        } else {
            RandomPlayer.select_move(ctx, board, moves, rng)
        }
    }
}

impl NamedPlayer for StatefulGetItOverWith {
    fn name(&self) -> Cow<'static, str> {
        Cow::from("GetItOverWith")
    }
}

#[derive(Clone)]
pub struct DilutedPlayer<P: StrugglePlayer>(pub P, pub f64);

impl<P: StrugglePlayer> StrugglePlayer for DilutedPlayer<P> {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &Board,
        moves: &'a [StruggleMove],
        rng: &mut SmallRng,
    ) -> &'a StruggleMove {
        if rng.gen::<f64>() < self.1 {
            self.0.select_move(ctx, board, moves, rng)
        } else {
            moves.choose(rng).unwrap()
        }
    }
}

impl<P: StrugglePlayer> NamedPlayer for DilutedPlayer<P> {
    fn name(&self) -> Cow<'static, str> {
        Cow::from(format!("{} {:.0}%", self.0.name(), self.1 * 100.0))
    }
}

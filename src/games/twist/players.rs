use std::borrow::Cow;

use itertools::Itertools;
use rand::{
    rngs::SmallRng,
    seq::{IteratorRandom, SliceRandom},
};

use crate::{
    game::NamedPlayer,
    games::struggle::{board::PiecePosition, PlayerColor},
};

use super::board::{ActionDieMove, DieResult, MoveFrom, NumberDieMove, TwistBoard, TwistMove};

pub trait TwistPlayer: Clone + Send + Sync + NamedPlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        rng: &mut SmallRng,
    ) -> &'a TwistMove;
}

#[derive(Clone)]
pub struct GameContext {
    pub die: DieResult,

    pub current_player: PlayerColor,
    pub other_player: PlayerColor,
}

impl GameContext {
    pub fn with_swapped_players(&self) -> Self {
        let mut ctx = self.clone();
        std::mem::swap(&mut ctx.current_player, &mut ctx.other_player);
        ctx
    }
}

#[derive(Clone)]
/// Plays completely randomly.
pub struct TwistRandomPlayer;

impl NamedPlayer for TwistRandomPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Random")
    }
}

impl TwistPlayer for TwistRandomPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &TwistBoard,
        moves: &'a [TwistMove],
        rng: &mut SmallRng,
    ) -> &'a TwistMove {
        moves.choose(rng).unwrap()
    }
}

#[derive(Clone)]
/// Always plays the default move (do nothing).
pub struct TwistDoNothingPlayer;

impl NamedPlayer for TwistDoNothingPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Do Nothing")
    }
}

impl TwistPlayer for TwistDoNothingPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &TwistBoard,
        _moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        let default_move = TwistMove::default();
        _moves.iter().find(|m| m == &&default_move).unwrap()
    }
}

#[derive(Clone)]
/// Plays randomly, but always tries to do something.
pub struct TwistDoSomethingPlayer;

impl NamedPlayer for TwistDoSomethingPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Do Something")
    }
}

impl TwistPlayer for TwistDoSomethingPlayer {
    fn select_move<'a>(
        &mut self,
        _ctx: &GameContext,
        _board: &TwistBoard,
        moves: &'a [TwistMove],
        rng: &mut SmallRng,
    ) -> &'a TwistMove {
        if moves.len() == 1 {
            return moves.first().unwrap();
        }

        let default_move = TwistMove::default();

        moves
            .iter()
            .filter(|m| *m != &default_move)
            .choose(rng)
            .unwrap()
    }
}

fn score_move(mov: &TwistMove, board: &TwistBoard, ctx: &GameContext) -> i32 {
    let mut score = 0;
    score += match &mov.0 {
        NumberDieMove::MovePiece { from, eats, .. } => {
            let adding_new_piece_score = match from {
                MoveFrom::Home => 400,
                MoveFrom::Board(_) => 10,
            };

            let eats_score = if *eats { 200 } else { 0 };

            adding_new_piece_score + eats_score
        }
        NumberDieMove::MoveToGoal { .. } => 500,
        NumberDieMove::DoNothing => -200,
    };
    let mut board_after_move = board.clone();
    board_after_move.perform_move(
        ctx.current_player,
        &TwistMove(mov.0.clone(), ActionDieMove::DoNothing),
    );
    score += match &mov.1 {
        ActionDieMove::SpinSection(section) => {
            let section = board_after_move.get_spin_section(*section);

            fn score_spin_section(
                current_player: PlayerColor,
                spin_section: &[Option<PlayerColor>; 5],
            ) -> i32 {
                let weights = [-3, -2, 0, 4, 6];
                weights
                    .into_iter()
                    .zip(spin_section.iter())
                    .fold(0, |acc, (weight, x)| {
                        acc + weight
                            * match x {
                                Some(player) if *player == current_player => 1,
                                Some(_) => -1,
                                None => 0,
                            }
                    })
            }

            let before = score_spin_section(ctx.current_player, section);
            let mut rotated_section = section.clone();
            rotated_section.reverse();
            let after = score_spin_section(ctx.current_player, &rotated_section);

            after - before
        }
        // TODO: Implement heuristic for RotateBoard?
        ActionDieMove::RotateBoard => -1000,
        ActionDieMove::DoNothing => 0,
    };
    score
}

#[derive(Clone)]
/// Scores different moves based on simple heuristics and plays the best one.
pub struct TwistScoreMovePlayer;

impl NamedPlayer for TwistScoreMovePlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Score Move")
    }
}

impl TwistPlayer for TwistScoreMovePlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        moves
            .iter()
            // Filter out moves that rotate the board, since they are not supported by the heuristic.
            .filter(|mov| mov.1 != ActionDieMove::RotateBoard)
            .sorted_by_cached_key(|mov| -score_move(mov, board, ctx))
            .next()
            .unwrap()
    }
}

#[derive(Clone)]
/// Scores different moves based on simple heuristics and plays the worst one.
pub struct TwistWorstScoreMovePlayer;

impl NamedPlayer for TwistWorstScoreMovePlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Worst Score Move")
    }
}

impl TwistPlayer for TwistWorstScoreMovePlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        moves
            .iter()
            .filter(|mov| mov.1 != ActionDieMove::RotateBoard)
            .sorted_by_cached_key(|mov| score_move(mov, board, ctx))
            .next()
            .unwrap()
    }
}

fn score_board(board: &TwistBoard, ctx: &GameContext) -> i32 {
    let mut score = 0;

    match board.get_winner() {
        Some(winner) if winner == ctx.current_player => {
            return 100_000;
        }
        Some(_) => {
            return -100_000;
        }
        _ => {}
    }

    let (pieces, enemy_pieces) = board.get_pieces(ctx.current_player);

    fn score_piece(board: &TwistBoard, ctx: &GameContext, piece: &PiecePosition) -> i32 {
        let my_home = TwistBoard::get_start(ctx.current_player);
        let enemy_home = TwistBoard::get_start(ctx.other_player);

        let mut score = 0i32;

        match piece {
            PiecePosition::Board(pos) => {
                score += 100;

                // Discourage staying in home base because it prevents spawning new pieces.
                if *pos == my_home {
                    score -= 50;
                }
                // REALLY discourage going to enemy home base because the piece is vulnerable.
                else if *pos == enemy_home
                    && board.home_bases[ctx.other_player as usize].pieces_waiting > 0
                {
                    score -= 200;
                } else {
                    score -= board.distance_to_goal(ctx.current_player, *pos) as i32;
                }
            }
            PiecePosition::Goal(_) => {
                score += 1000;
            }
        };

        score
    }

    score += pieces
        .iter()
        .map(|piece| score_piece(board, ctx, piece))
        .sum::<i32>();

    let enemy_ctx = ctx.with_swapped_players();

    score -= enemy_pieces
        .iter()
        .map(|piece| score_piece(board, &enemy_ctx, piece))
        .sum::<i32>();

    score
}

#[derive(Clone)]
/// Scores different moves by scoring the board after the move and plays the best one.
/// Similar to Expectiminimax but only looks one move ahead, so it's not really a minimax.
pub struct TwistScoreBoardPlayer;

impl NamedPlayer for TwistScoreBoardPlayer {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Score Board")
    }
}

impl TwistPlayer for TwistScoreBoardPlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        /*println!("{:?}", ctx.die);
        println!("{:?} scoring {} moves", ctx.current_player, moves.len());*/

        let mov = moves
            .iter()
            .sorted_by_cached_key(|mov| {
                let mut board_after_move = board.clone();
                board_after_move.perform_move(ctx.current_player, mov);

                let score = score_board(&board_after_move, ctx);

                //println!("{:?} {:?} -> {}", ctx.current_player, mov, score);

                -score
            })
            .next()
            .unwrap();

        //println!("{:?} selected {:?}\n", ctx.current_player, mov);

        mov
    }
}

#[derive(Clone)]
pub struct TwistScoreBoardPlayerWorst;

impl NamedPlayer for TwistScoreBoardPlayerWorst {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Score Board Worst")
    }
}

impl TwistPlayer for TwistScoreBoardPlayerWorst {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        moves
            .iter()
            .sorted_by_cached_key(|mov| {
                let mut board_after_move = board.clone();
                board_after_move.perform_move(ctx.current_player, mov);

                let score = score_board(&board_after_move, ctx);

                score
            })
            .next()
            .unwrap()
    }
}

// Heuristic for scoring boards which maximizes game length, not winning.
fn score_board_max_length(board: &TwistBoard, ctx: &GameContext) -> i32 {
    let mut score = 0;

    // Winning or losing is bad.
    match board.get_winner() {
        Some(_) => {
            return -100_000;
        }
        _ => {}
    }

    let (pieces, enemy_pieces) = board.get_pieces(ctx.current_player);

    fn score_piece(board: &TwistBoard, ctx: &GameContext, piece: &PiecePosition) -> i32 {
        let my_home = TwistBoard::get_start(ctx.current_player);
        let enemy_home = TwistBoard::get_start(ctx.other_player);

        let mut score = 0i32;

        match piece {
            PiecePosition::Board(pos) => {
                score += 100;

                // Encourage getting to enemy home base to get eaten.
                if *pos == enemy_home {
                    score += 200;
                } else if *pos == my_home {
                    // Discourage staying in home base because it prevents spawning new pieces.
                    score -= 50;
                } else {
                    score -= board.distance_to_goal(ctx.current_player, *pos) as i32;
                }
            }
            PiecePosition::Goal(_) => {
                score -= 1000;
            }
        };

        score
    }

    score += pieces
        .iter()
        .map(|piece| score_piece(board, ctx, piece))
        .sum::<i32>();

    let enemy_ctx = ctx.with_swapped_players();

    score += enemy_pieces
        .iter()
        .map(|piece| score_piece(board, &enemy_ctx, piece))
        .sum::<i32>();

    score
}

#[derive(Clone)]
pub struct TwistScoreBoardPlayerMaximizeLength;

impl NamedPlayer for TwistScoreBoardPlayerMaximizeLength {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Score Board Maximize Length")
    }
}

impl TwistPlayer for TwistScoreBoardPlayerMaximizeLength {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        _rng: &mut SmallRng,
    ) -> &'a TwistMove {
        moves
            .iter()
            .sorted_by_cached_key(|mov| {
                let mut board_after_move = board.clone();
                board_after_move.perform_move(ctx.current_player, mov);

                let score = score_board_max_length(&board_after_move, ctx);

                -score
            })
            .next()
            .unwrap()
    }
}

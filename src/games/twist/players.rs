use std::borrow::Cow;

use rand::{rngs::SmallRng, seq::SliceRandom};

use crate::{game::NamedPlayer, games::struggle::PlayerColor};

use super::board::{DieResult, TwistBoard, TwistMove};

pub trait TwistPlayer: Clone + Send + Sync + NamedPlayer {
    fn select_move<'a>(
        &mut self,
        ctx: &GameContext,
        board: &TwistBoard,
        moves: &'a [TwistMove],
        rng: &mut SmallRng,
    ) -> &'a TwistMove;
}

pub struct GameContext {
    pub die: DieResult,

    pub current_player: PlayerColor,
    pub other_player: PlayerColor,
}

#[derive(Clone)]
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

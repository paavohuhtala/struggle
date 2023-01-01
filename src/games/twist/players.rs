use std::borrow::Cow;

use rand::rngs::SmallRng;

use super::board::{DieResult, TwistBoard, TwistMove};

pub trait TwistPlayer: Clone + Send + Sync {
    fn name(&self) -> Cow<'static, str>;

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
}

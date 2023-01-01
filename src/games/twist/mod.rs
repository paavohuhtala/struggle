use rand::Rng;

use crate::game::{RaceGame, TurnResult};

use self::{
    board::{ActionDie, DieResult, TwistBoard, TwistMove, TwistMoveVec},
    get_moves::get_twist_moves,
    players::{GameContext, TwistPlayer},
};

use super::struggle::{AiStrugglePlayer, PlayerColor};

pub mod board;
pub mod get_moves;
pub mod players;

pub struct TwistGameStats {
    pub move_distribution: [[u16; 25]; 2],
    pub turns: u16,
}

pub struct TwistGame<A: TwistPlayer, B: TwistPlayer> {
    board: TwistBoard,
    player_a: AiStrugglePlayer<A>,
    player_b: AiStrugglePlayer<B>,

    current_player: PlayerColor,

    stats: Option<TwistGameStats>,
}

impl<A: TwistPlayer, B: TwistPlayer> RaceGame for TwistGame<A, B> {
    type Board = TwistBoard;
    type PlayerId = PlayerColor;

    type Move = TwistMove;
    type MoveVector = TwistMoveVec;

    type TurnContext = players::GameContext;

    type DiceState = DieResult;

    fn board(&self) -> &Self::Board {
        &self.board
    }

    fn current_player(&self) -> Self::PlayerId {
        self.current_player
    }

    fn other_player(&self) -> Self::PlayerId {
        if self.player_a.color == self.current_player {
            self.player_b.color
        } else {
            self.player_a.color
        }
    }

    fn set_current_player(&mut self, player: Self::PlayerId) {
        self.current_player = player;
    }

    fn throw_dice(rng: &mut rand::rngs::SmallRng) -> Self::DiceState {
        let number = rng.gen_range(1..=6);
        let action = ActionDie::get_random(rng);
        DieResult { number, action }
    }

    fn create_turn_context(&self, die: Self::DiceState) -> Self::TurnContext {
        GameContext { die }
    }

    fn get_moves(&self, ctx: &Self::TurnContext) -> Self::MoveVector {
        get_twist_moves(
            &self.board,
            ctx.die.clone(),
            self.current_player,
            self.other_player(),
        )
    }

    fn apply_move(
        &mut self,
        ctx: &Self::TurnContext,
        mov: &Self::Move,
    ) -> crate::game::TurnResult<Self::PlayerId> {
        self.board.perform_move(self.current_player, mov);
        if let Some(winner) = self.board.get_winner() {
            TurnResult::EndGame { winner }
        } else {
            if ctx.die.number == 6 {
                TurnResult::PlayAgain
            } else {
                TurnResult::PassTo(self.other_player())
            }
        }
    }

    fn select_move<'a>(
        &mut self,
        _ctx: &Self::TurnContext,
        moves: &'a Self::MoveVector,
        _rng: &mut rand::rngs::SmallRng,
    ) -> &'a Self::Move {
        if let Some(stats) = &mut self.stats {
            stats.turns += 1;
            stats.move_distribution[self.current_player as usize][moves.len() - 1] += 1;
        }

        todo!()
    }
}

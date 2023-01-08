use rand::{seq::SliceRandom, Rng};

use crate::game::{CreateGame, GameStats, IntoGameStats, RaceGame, TurnResult};

use self::{
    board::{ActionDie, DieResult, TwistBoard, TwistMove, TwistMoveVec},
    get_moves::get_twist_moves,
    players::{GameContext, TwistPlayer},
};

use super::struggle::{AiStrugglePlayer, PlayerColor};

pub mod board;
pub mod get_moves;
pub mod players;

pub type TwistGameStats = GameStats<25>;

pub struct TwistGame<A: TwistPlayer, B: TwistPlayer> {
    board: TwistBoard,
    player_a: AiStrugglePlayer<A>,
    player_b: AiStrugglePlayer<B>,

    current_player: PlayerColor,

    stats: Option<TwistGameStats>,
}

impl<A: TwistPlayer, B: TwistPlayer> TwistGame<A, B> {
    pub fn new(
        player_a: AiStrugglePlayer<A>,
        player_b: AiStrugglePlayer<B>,
        collect_stats: bool,
    ) -> Self {
        let board = TwistBoard::new((player_a.color, player_b.color));

        Self {
            board,
            current_player: player_a.color,
            player_a,
            player_b,
            stats: collect_stats.then(|| TwistGameStats::default()),
        }
    }
}

impl<A: TwistPlayer, B: TwistPlayer> RaceGame for TwistGame<A, B> {
    type Board = TwistBoard;
    type PlayerId = PlayerColor;

    type Move = TwistMove;
    type MoveVector = TwistMoveVec;

    type TurnContext = players::GameContext;

    type DiceState = DieResult;

    const MAX_MOVES: usize = 25;

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
        GameContext {
            die,
            current_player: self.current_player,
            other_player: self.other_player(),
        }
    }

    fn get_moves(&self, ctx: &Self::TurnContext) -> Self::MoveVector {
        get_twist_moves(&self.board, ctx.die.clone(), self.current_player)
    }

    fn apply_move(
        &mut self,
        ctx: &Self::TurnContext,
        mov: &Self::Move,
    ) -> crate::game::TurnResult<Self::PlayerId> {
        self.board.perform_move(self.current_player, mov);

        if let Some(winner) = self.board.get_winner() {
            TurnResult::EndGame { winner }
        } else if ctx.die.number == 6 {
            TurnResult::PlayAgain
        } else {
            TurnResult::PassTo(self.other_player())
        }
    }

    fn select_move<'a>(
        &mut self,
        ctx: &Self::TurnContext,
        moves: &'a Self::MoveVector,
        rng: &mut rand::rngs::SmallRng,
    ) -> &'a Self::Move {
        if let Some(stats) = &mut self.stats {
            let index = if self.current_player == self.player_a.color {
                0
            } else {
                1
            };

            stats.turns += 1;
            stats.move_distribution[index][moves.len() - 1] += 1;
        }

        if self.current_player == self.player_a.color {
            self.player_a
                .player
                .select_move(ctx, &self.board, moves, rng)
        } else {
            self.player_b
                .player
                .select_move(ctx, &self.board, moves, rng)
        }
    }

    fn play_turn(
        &mut self,
        rng: &mut rand::rngs::SmallRng,
    ) -> (Self::DiceState, TurnResult<Self::PlayerId>) {
        let dice = Self::throw_dice(rng);
        let ctx = self.create_turn_context(dice.clone());

        let mut moves = self.get_moves(&ctx);
        moves.shuffle(rng);

        let mov = self.select_move(&ctx, &moves, rng);

        (dice, self.apply_move(&ctx, mov))
    }
}

impl<A: TwistPlayer, B: TwistPlayer> CreateGame for TwistGame<A, B> {
    type PlayerA = A;
    type PlayerB = B;

    fn create_game(
        player_a: (PlayerColor, A),
        player_b: (PlayerColor, B),
        collect_stats: bool,
    ) -> Self {
        let player_a = AiStrugglePlayer::new(player_a.0, player_a.1);
        let player_b = AiStrugglePlayer::new(player_b.0, player_b.1);

        Self::new(player_a, player_b, collect_stats)
    }
}

impl<A: TwistPlayer, B: TwistPlayer> IntoGameStats<25> for TwistGame<A, B> {
    fn into_stats(self) -> Option<TwistGameStats> {
        self.stats
    }
}

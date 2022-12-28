use arrayvec::ArrayVec;
use rand::{prelude::SmallRng, Rng, SeedableRng};

use crate::{players, struggle, GameStats};

#[derive(Debug)]
pub enum TurnResult<PlayerId> {
    PlayAgain,
    PassTo(PlayerId),
    EndGame { winner: PlayerId },
}

pub trait RaceGame {
    type Board;
    type PlayerId;

    type Move;
    type MoveVector;

    type TurnContext;
    type DiceState;

    fn board(&self) -> &Self::Board;
    fn current_player(&self) -> Self::PlayerId;
    fn other_player(&self) -> Self::PlayerId;
    fn set_current_player(&mut self, player: Self::PlayerId);

    fn throw_dice(rng: &mut SmallRng) -> Self::DiceState;

    fn create_turn_context(&self, dice: Self::DiceState) -> Self::TurnContext;

    fn get_moves(&self, ctx: &Self::TurnContext) -> Self::MoveVector;

    fn apply_move(
        &mut self,
        ctx: &Self::TurnContext,
        mov: &Self::Move,
    ) -> TurnResult<Self::PlayerId>;

    fn select_move<'a>(
        &mut self,
        ctx: &Self::TurnContext,
        moves: &'a Self::MoveVector,
        rng: &mut SmallRng,
    ) -> &'a Self::Move;

    fn play_turn(&mut self, rng: &mut SmallRng) -> TurnResult<Self::PlayerId> {
        let dice = Self::throw_dice(rng);
        let ctx = self.create_turn_context(dice);

        let moves = self.get_moves(&ctx);
        let mov = self.select_move(&ctx, &moves, rng);

        self.apply_move(&ctx, mov)
    }
}

pub fn play_game<G: RaceGame>(game: &mut G) -> G::PlayerId {
    let rng = &mut SmallRng::from_rng(rand::thread_rng()).unwrap();
    loop {
        match game.play_turn(rng) {
            TurnResult::PlayAgain => {}
            TurnResult::PassTo(player) => {
                game.set_current_player(player);
            }
            TurnResult::EndGame { winner } => {
                return winner;
            }
        }
    }
}

#[derive(Clone)]
pub struct AiStrugglePlayer<T: players::StrugglePlayer> {
    color: struggle::PlayerColor,
    player: T,
}

impl<T: players::StrugglePlayer> AiStrugglePlayer<T> {
    pub fn new(color: struggle::PlayerColor, player: T) -> Self {
        Self { color, player }
    }

    pub fn color(&self) -> struggle::PlayerColor {
        self.color
    }
}

pub struct StruggleGame<A: players::StrugglePlayer, B: players::StrugglePlayer> {
    board: struggle::Board,
    player_a: AiStrugglePlayer<A>,
    player_b: AiStrugglePlayer<B>,

    current_player: struggle::PlayerColor,

    stats: Option<GameStats>,
}

impl<A: players::StrugglePlayer, B: players::StrugglePlayer> StruggleGame<A, B> {
    pub fn new(
        player_a: AiStrugglePlayer<A>,
        player_b: AiStrugglePlayer<B>,
        collect_stats: bool,
    ) -> Self {
        let board = struggle::Board::new(player_a.color, player_b.color);

        Self {
            board,
            current_player: player_a.color,
            player_a,
            player_b,
            stats: collect_stats.then(|| GameStats::default()),
        }
    }

    pub fn into_stats(self) -> Option<GameStats> {
        self.stats
    }
}

impl<A: players::StrugglePlayer, B: players::StrugglePlayer> RaceGame for StruggleGame<A, B> {
    type Board = struggle::Board;
    type PlayerId = struggle::PlayerColor;

    type Move = struggle::ValidMove;
    type MoveVector = ArrayVec<struggle::ValidMove, 4>;

    type TurnContext = players::GameContext;
    type DiceState = u8;

    fn board(&self) -> &struggle::Board {
        &self.board
    }

    fn current_player(&self) -> struggle::PlayerColor {
        self.current_player
    }

    fn other_player(&self) -> struggle::PlayerColor {
        if self.player_a.color == self.current_player {
            self.player_b.color
        } else {
            self.player_a.color
        }
    }

    fn set_current_player(&mut self, player: struggle::PlayerColor) {
        self.current_player = player;
    }

    fn throw_dice(rng: &mut SmallRng) -> u8 {
        rng.gen_range(1..=6)
    }

    fn create_turn_context(&self, dice: u8) -> Self::TurnContext {
        Self::TurnContext {
            current_player: self.current_player,
            other_player: self.other_player(),
            dice,
        }
    }

    fn get_moves(&self, ctx: &Self::TurnContext) -> Self::MoveVector {
        self.board
            .get_moves(ctx.dice, ctx.current_player, ctx.other_player)
    }

    fn select_move<'a>(
        &mut self,
        ctx: &Self::TurnContext,
        moves: &'a Self::MoveVector,
        rng: &mut SmallRng,
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

    fn apply_move(
        &mut self,
        ctx: &Self::TurnContext,
        mov: &Self::Move,
    ) -> TurnResult<Self::PlayerId> {
        self.board.perform_move(ctx.current_player, mov);

        if let Some(winner) = self.board.get_winner() {
            TurnResult::EndGame { winner }
        } else if ctx.dice == 6 {
            TurnResult::PlayAgain
        } else {
            TurnResult::PassTo(self.other_player())
        }
    }
}

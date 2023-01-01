use arrayvec::ArrayVec;
use rand::{rngs::SmallRng, Rng};

use crate::game::{RaceGame, TurnResult};

use self::board::{Board, StruggleMove};

pub mod board;
pub mod players;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlayerColor {
    Red = 0,
    Blue,
    Yellow,
    Green,
}

impl From<usize> for PlayerColor {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Red,
            1 => Self::Blue,
            2 => Self::Yellow,
            3 => Self::Green,
            _ => panic!("Invalid player color"),
        }
    }
}

pub const COLORS: [PlayerColor; 4] = [
    PlayerColor::Red,
    PlayerColor::Blue,
    PlayerColor::Yellow,
    PlayerColor::Green,
];

#[derive(Debug, Default)]
pub struct GameStats {
    pub move_distribution: [[u16; 4]; 2],
    pub turns: u16,
}

#[derive(Clone)]
pub struct AiStrugglePlayer<T> {
    pub color: PlayerColor,
    player: T,
}

impl<T> AiStrugglePlayer<T> {
    pub fn new(color: PlayerColor, player: T) -> Self {
        Self { color, player }
    }

    pub fn color(&self) -> PlayerColor {
        self.color
    }
}

pub struct StruggleGame<A: players::StrugglePlayer, B: players::StrugglePlayer> {
    board: Board,
    player_a: AiStrugglePlayer<A>,
    player_b: AiStrugglePlayer<B>,

    current_player: PlayerColor,

    stats: Option<GameStats>,
}

impl<A: players::StrugglePlayer, B: players::StrugglePlayer> StruggleGame<A, B> {
    pub fn new(
        player_a: AiStrugglePlayer<A>,
        player_b: AiStrugglePlayer<B>,
        collect_stats: bool,
    ) -> Self {
        let board = Board::new(player_a.color, player_b.color);

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
    type Board = Board;
    type PlayerId = PlayerColor;

    type Move = StruggleMove;
    type MoveVector = ArrayVec<StruggleMove, 4>;

    type TurnContext = players::GameContext;
    type DiceState = u8;

    fn board(&self) -> &Board {
        &self.board
    }

    fn current_player(&self) -> PlayerColor {
        self.current_player
    }

    fn other_player(&self) -> PlayerColor {
        if self.player_a.color == self.current_player {
            self.player_b.color
        } else {
            self.player_a.color
        }
    }

    fn set_current_player(&mut self, player: PlayerColor) {
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

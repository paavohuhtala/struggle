use std::{borrow::Cow, fmt::Debug};

use rand::{prelude::SmallRng, Rng, SeedableRng};

#[derive(Debug)]
pub enum TurnResult<PlayerId> {
    PlayAgain,
    PassTo(PlayerId),
    EndGame { winner: PlayerId },
}

#[derive(Debug, Clone)]
pub struct GameStats<const MAX_MOVES: usize> {
    pub move_distribution: [[u16; MAX_MOVES]; 2],
    pub pieces_eaten_by: [u16; 2],
    pub turns: u16,
}

impl<const MAX_MOVES: usize> GameStats<MAX_MOVES> {
    pub fn new() -> Self {
        Self {
            move_distribution: [[0; MAX_MOVES]; 2],
            pieces_eaten_by: [0; 2],
            turns: 0,
        }
    }
}

impl<const MAX_MOVES: usize> Default for GameStats<MAX_MOVES> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait NamedPlayer {
    fn name(&self) -> Cow<'static, str>;
}

pub trait RaceGame {
    type Board;
    type PlayerId: Debug + Send + Sync + Clone + Eq + PartialEq;

    type Move: Debug;
    type MoveVector;

    type TurnContext;
    type DiceState: Clone + Debug;

    const MAX_MOVES: usize;

    fn board(&self) -> &Self::Board;
    fn current_player(&self) -> Self::PlayerId;
    fn other_player(&self) -> Self::PlayerId;
    fn set_current_player(&mut self, player: Self::PlayerId);

    fn throw_dice(&self, rng: &mut SmallRng) -> Self::DiceState;

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

    fn play_turn(&mut self, rng: &mut SmallRng) -> (Self::DiceState, TurnResult<Self::PlayerId>)
    where
        Self::Move: Debug,
    {
        let dice = self.throw_dice(rng);
        (dice.clone(), self.play_turn_with_die(dice, rng))
    }

    fn play_turn_with_die(
        &mut self,
        dice: Self::DiceState,
        rng: &mut SmallRng,
    ) -> TurnResult<Self::PlayerId>
    where
        Self::Move: Debug,
    {
        let ctx = self.create_turn_context(dice.clone());
        let moves = self.get_moves(&ctx);
        let mov = self.select_move(&ctx, &moves, rng);

        /*println!(
            "{:?} plays {:?} with dice {:?}",
            self.current_player(),
            mov,
            dice
        );*/

        self.apply_move(&ctx, mov)
    }
}

pub trait CreateGame: RaceGame {
    type PlayerA: NamedPlayer + Clone + Send + Sync;
    type PlayerB: NamedPlayer + Clone + Send + Sync;

    fn create_game(
        player_a: (Self::PlayerId, Self::PlayerA),
        player_b: (Self::PlayerId, Self::PlayerB),
        collect_stats: bool,
    ) -> Self;
}

pub fn play_game<G: RaceGame>(game: &mut G) -> G::PlayerId {
    let rng = &mut SmallRng::from_rng(rand::thread_rng()).unwrap();

    // Randomly select who starts
    if rng.gen() {
        game.set_current_player(game.other_player());
    }

    loop {
        match game.play_turn(rng).1 {
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

pub trait IntoGameStats<const MAX_MOVES: usize>: RaceGame {
    fn into_stats(self) -> Option<GameStats<MAX_MOVES>>;
}

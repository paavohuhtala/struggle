use rand::{prelude::SmallRng, SeedableRng};

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
    type DiceState: Clone;

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

    fn play_turn(&mut self, rng: &mut SmallRng) -> (Self::DiceState, TurnResult<Self::PlayerId>) {
        let dice = Self::throw_dice(rng);
        let ctx = self.create_turn_context(dice.clone());

        let moves = self.get_moves(&ctx);
        let mov = self.select_move(&ctx, &moves, rng);

        (dice, self.apply_move(&ctx, mov))
    }
}

pub fn play_game<G: RaceGame>(game: &mut G) -> G::PlayerId {
    let rng = &mut SmallRng::from_rng(rand::thread_rng()).unwrap();
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

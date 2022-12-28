pub mod game;
pub mod players;
pub mod struggle;

use players::{GameContext, StrugglePlayer};
use rand::prelude::*;
use struggle::{Board, PlayerColor};

#[derive(Debug, Default)]
pub struct GameStats {
    pub move_distribution: [[u16; 4]; 2],
    pub turns: u16,
}

#[derive(Debug)]
pub struct GameResult {
    pub winner: PlayerColor,
    pub stats: Option<Box<GameStats>>,
}

pub fn play_game<A, B>(
    mut player_a: (PlayerColor, A),
    mut player_b: (PlayerColor, B),
    collect_stats: bool,
) -> GameResult
where
    A: StrugglePlayer,
    B: StrugglePlayer,
{
    let mut rng = SmallRng::from_rng(rand::thread_rng()).unwrap();

    let player_a_color = player_a.0;

    // randomize first player
    let (mut current_player, mut other_player) = if rng.gen::<bool>() {
        (player_b.0, player_a.0)
    } else {
        (player_a.0, player_b.0)
    };

    let mut board = Board::new(player_a.0, player_b.0);

    let mut stats = collect_stats.then(GameStats::default);

    loop {
        let dice = rng.gen_range(1..=6);

        let ctx = GameContext {
            current_player,
            other_player,
            dice,
        };

        let moves = board.get_moves(dice, current_player, other_player);

        if let Some(stats) = stats.as_mut() {
            let index = if current_player == player_a_color {
                0
            } else {
                1
            };

            stats.turns += 1;
            stats.move_distribution[index][moves.len() as usize - 1] += 1;
        }

        let mov = if moves.len() == 1 {
            &moves[0]
        } else if current_player == player_a_color {
            player_a.1.select_move(&ctx, &board, &moves, &mut rng)
        } else {
            player_b.1.select_move(&ctx, &board, &moves, &mut rng)
        }
        .clone();

        board.perform_move(current_player, &mov);

        if let Some(winner) = board.get_winner() {
            return GameResult {
                winner,
                stats: stats.map(Box::new),
            };
        }

        if dice != 6 {
            std::mem::swap(&mut current_player, &mut other_player);
        }
    }
}

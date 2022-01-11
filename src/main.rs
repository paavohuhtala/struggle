use ::rand::prelude::*;
use itertools::Itertools;
use rayon::prelude::*;
use struggle_core::{
    players::{expectimax, GameContext, StrugglePlayer},
    struggle::{Board, Player},
};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug, Default)]
struct GameStats {
    pub turns: [u16; 2],
    pub move_distribution: [[u16; 4]; 2],
}

#[derive(Debug)]
struct GameResult {
    pub winner: Player,
    pub stats: Option<Box<GameStats>>,
}

fn play_game<'a, A, B>(
    player_a: (Player, &'a mut A),
    player_b: (Player, &'a mut B),
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

    let mut board = Board::new();

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

            stats.turns[index] += 1;
            stats.move_distribution[index][moves.len() as usize - 1] += 1;
        }

        let mov = if current_player == player_a_color {
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

fn print_move_distribution_graph(distribution: [u32; 4]) {
    println!("{:?}", distribution);

    let max = distribution.iter().copied().max().unwrap();

    for i in 0..4 {
        let bar_length = (distribution[i] as f32 / max as f32) * 20.0;
        let bar = std::iter::repeat('#')
            .take(bar_length as usize)
            .collect::<String>();
        println!("{:>2}: {}", i + 1, bar);
    }
}

pub fn main() {
    let a_color = Player::Blue;
    let b_color = Player::Yellow;

    let total_games = 100_000u32;

    let collect_stats = false;

    let results = (0..total_games)
        .into_par_iter()
        .map(|_| {
            let mut player_a = expectimax(2);
            let mut player_b = expectimax(2);
            play_game(
                (a_color, &mut player_a),
                (b_color, &mut player_b),
                collect_stats,
            )
        })
        .collect::<Vec<_>>();

    let total_a_wins = results.iter().fold(0, |acc, result| {
        if result.winner == a_color {
            acc + 1
        } else {
            acc
        }
    });

    let total_games = results.len();
    let total_b_wins = total_games - total_a_wins;

    let a_b_win_ratio = total_a_wins as f64 / total_games as f64;
    let difference_percentage = (a_b_win_ratio - 0.5).abs() * 100.0;
    let diff_word = if a_b_win_ratio > 0.5 { "more" } else { "less" };

    println!(
        "{} games, player A won {}, player B won {}",
        total_games, total_a_wins, total_b_wins
    );
    println!(
        "p(a_wins) = {:.2} ({:.1}% {})",
        a_b_win_ratio, difference_percentage, diff_word
    );

    if collect_stats {
        let stats = results
            .iter()
            .map(|r| r.stats.as_ref().unwrap().as_ref())
            .collect_vec();

        let average_length = stats
            .iter()
            .map(|s| s.turns[0] as f64 + s.turns[1] as f64)
            .sum::<f64>()
            / stats.len() as f64;

        let mut turns_per_player = [0.0, 0.0];

        for s in stats.iter() {
            turns_per_player[0] += s.turns[0] as f64;
            turns_per_player[1] += s.turns[1] as f64;
        }

        turns_per_player[0] /= stats.len() as f64;
        turns_per_player[1] /= stats.len() as f64;

        println!("avg total turns: {:?}", average_length);
        println!("avg turns per player: {:?}", turns_per_player);

        let mut move_distribution = [[0, 0, 0, 0]; 2];

        for s in stats.iter() {
            for i in 0..4 {
                move_distribution[0][i] += s.move_distribution[0][i] as u32;
                move_distribution[1][i] += s.move_distribution[1][i] as u32;
            }
        }

        let choice_percentage_a = move_distribution[0][1..3]
            .iter()
            .map(|&i| i as f64)
            .sum::<f64>()
            / move_distribution[0].iter().map(|&i| i as f64).sum::<f64>()
            * 100.0;

        let choice_percentage_b = move_distribution[1][1..3]
            .iter()
            .map(|&i| i as f64)
            .sum::<f64>()
            / move_distribution[1].iter().map(|&i| i as f64).sum::<f64>()
            * 100.0;

        println!("move count distribution:");

        println!("player 1:");
        print_move_distribution_graph(move_distribution[0]);
        println!("{:.1}% of turns had choices", choice_percentage_a);

        println!("player 2:");
        print_move_distribution_graph(move_distribution[1]);
        println!("{:.1}% of turns had choices", choice_percentage_b);
    }
}

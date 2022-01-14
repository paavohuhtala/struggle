use ::rand::prelude::*;
use itertools::Itertools;
use rayon::prelude::*;
use struggle_core::{
    players::{
        confused_expectimax, expectimax, maximize_options_expectimax, minimize_options_expectimax,
        participatory_expectimax, worst_expectimax, GameContext, RandomDietPlayer,
        RandomEaterPlayer, RandomPlayer, StrugglePlayer,
    },
    struggle::{Board, Player},
};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug, Default)]
struct GameStats {
    pub move_distribution: [[u16; 4]; 2],
    pub turns: u16,
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

fn print_move_distribution_graph(distribution: [u32; 4]) {
    println!("{:?}", distribution);

    let total = distribution.iter().sum::<u32>();

    let max = distribution.iter().copied().max().unwrap();

    for (i, hits) in distribution.iter().enumerate() {
        let bar_length = (*hits as f32 / max as f32) * 50.0;
        let bar = "#".repeat(bar_length as usize);
        println!(
            "{:>2}: {:<50} ({:.1}%)",
            i + 1,
            bar,
            distribution[i] as f64 / total as f64 * 100.0
        );
    }
}

pub fn compare_players<A: StrugglePlayer, B: StrugglePlayer>(
    a: (Player, A),
    b: (Player, B),
    rounds: u32,
) -> f64 {
    let a_color = a.0;
    let b_color = b.0;

    let games_won_by_a = (0..rounds)
        .into_par_iter()
        .map(|_| {
            let mut player_a = a.1.clone();
            let mut player_b = b.1.clone();
            play_game((a_color, &mut player_a), (b_color, &mut player_b), false)
        })
        .filter(|res| res.winner == a_color)
        .count();

    games_won_by_a as f64 / rounds as f64
}

pub fn compare_players_detailed<A: StrugglePlayer, B: StrugglePlayer>(
    a: (Player, A),
    b: (Player, B),
    rounds: u32,
) {
    println!("{} vs {}", a.1.name(), b.1.name());

    let results = (0..rounds)
        .into_par_iter()
        .map(|_| {
            let mut player_a = a.1.clone();
            let mut player_b = b.1.clone();
            play_game((a.0, &mut player_a), (b.0, &mut player_b), true)
        })
        .collect::<Vec<_>>();

    let total_a_wins = results.iter().fold(
        0,
        |acc, result| {
            if result.winner == a.0 {
                acc + 1
            } else {
                acc
            }
        },
    );

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

    let stats = results
        .iter()
        .map(|r| r.stats.as_ref().unwrap().as_ref())
        .collect_vec();

    let average_length = stats.iter().map(|s| s.turns as f64).sum::<f64>() / stats.len() as f64;

    println!("avg total turns: {:?}", average_length);

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

    println!("{}", a.1.name());
    print_move_distribution_graph(move_distribution[0]);
    println!("{:.1}% of turns had choices", choice_percentage_a);

    println!("{}", b.1.name());
    print_move_distribution_graph(move_distribution[1]);
    println!("{:.1}% of turns had choices", choice_percentage_b);
}

const TOTAL_GAMES: u32 = 500_000;

macro_rules! run_games {
    ($($player_l: expr, [$($player_r: expr),*]),+) => {
        $(
            {
                let player_a = $player_l;

                $(
                    let player_b = $player_r;
                    let p_a = compare_players((Player::Red, player_a.clone()), (Player::Yellow, player_b.clone()), TOTAL_GAMES);
                    println!("{} vs {}: {}", player_a.name(), player_b.name(), p_a);
                )*
            }
        )+;
    };
}

pub fn main() {
    // let total_games = 50_000;

    /*compare_players_detailed(
        (Player::Red, expectimax(1)),
        (Player::Yellow, RandomPlayer),
        TOTAL_GAMES,
    );*/

    run_games!(
        RandomPlayer,
        [
            RandomPlayer,
            RandomEaterPlayer,
            RandomDietPlayer,
            expectimax(1),
            confused_expectimax(1),
            worst_expectimax(1),
            participatory_expectimax(1),
            maximize_options_expectimax(1),
            minimize_options_expectimax(1)
        ]
    );

    run_games!(
        RandomEaterPlayer,
        [
            RandomEaterPlayer,
            RandomDietPlayer,
            expectimax(1),
            confused_expectimax(1),
            worst_expectimax(1),
            participatory_expectimax(1),
            maximize_options_expectimax(1),
            minimize_options_expectimax(1)
        ]
    );

    run_games!(
        RandomDietPlayer,
        [
            RandomDietPlayer,
            expectimax(1),
            confused_expectimax(1),
            worst_expectimax(1),
            participatory_expectimax(1),
            maximize_options_expectimax(1),
            minimize_options_expectimax(1)
        ]
    );
}

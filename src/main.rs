use indicatif::ParallelProgressIterator;
use itertools::Itertools;
use plotters::prelude::*;
use rayon::prelude::*;
use struggle_core::{
    game::{play_game, CreateGame, IntoGameStats, NamedPlayer},
    games::{
        struggle::{
            players::{RandomPlayer, StrugglePlayer},
            PlayerColor, StruggleGame,
        },
        twist::{
            players::{TwistPlayer, TwistRandomPlayer},
            TwistGame,
        },
    },
};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn print_move_distribution_graph<const MAX_MOVES: usize>(distribution: [u32; MAX_MOVES]) {
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

fn wilson_score(p_hat: f64, samples: u64) -> (f64, f64) {
    let z: f64 = 1.96;

    let a = p_hat + z * z / (2.0 * samples as f64);
    let b =
        z * ((p_hat * (1.0 - p_hat) + z.powi(2) / (4.0 * samples as f64)) / samples as f64).sqrt();
    let c = 1.0 + z * z / samples as f64;

    ((a - b) / c, (a + b) / c)
}

pub fn compare_players_detailed<
    const MAX_MOVES: usize,
    G: CreateGame + IntoGameStats<MAX_MOVES>,
>(
    a: (G::PlayerId, G::PlayerA),
    b: (G::PlayerId, G::PlayerB),
    rounds: u32,
) {
    println!("{} ({:?}) vs {} ({:?})", a.1.name(), a.0, b.1.name(), b.0);

    let results = (0..rounds)
        .into_par_iter()
        .progress_count(rounds as u64)
        .map(|_| {
            let mut game = G::create_game(a.clone(), b.clone(), true);
            let winner = play_game(&mut game);
            (winner, game.into_stats().unwrap())
        })
        .collect::<Vec<_>>();

    let drawing_area = SVGBackend::new("length_distribution.svg", (1000, 500)).into_drawing_area();
    drawing_area.fill(&WHITE).unwrap();

    let total_games = results.len();
    let (winners, stats): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let turns = stats.iter().map(|stats| stats.turns as u32).collect_vec();
    let (&min_turns, &max_turns) = turns.iter().minmax().into_option().unwrap();
    let turn_counts = turns.iter().counts();
    let most_common_turn = turn_counts.values().copied().max().unwrap() as u32;

    let mut ctx = ChartBuilder::on(&drawing_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption(
            format!(
                "Game length distribution: {} vs {} (n={})",
                a.1.name(),
                b.1.name(),
                total_games
            ),
            ("Source Sans Pro, sans-serif", 20),
        )
        .build_cartesian_2d(
            ((min_turns - 2)..(max_turns + 2)).into_segmented(),
            0..(most_common_turn + 5),
        )
        .unwrap();

    ctx.configure_mesh().draw().unwrap();

    ctx.draw_series((min_turns..=max_turns).map(|i| {
        let count = *turn_counts.get(&i).unwrap_or(&0);
        let x0 = SegmentValue::Exact(i);
        let x1 = SegmentValue::Exact(i + 1);
        let bar = Rectangle::new([(x0, 0), (x1, count as u32)], GREEN.filled());
        bar
    }))
    .unwrap();

    let total_a_wins: usize = winners
        .into_par_iter()
        .fold(
            || 0,
            |acc, winner| {
                if winner == a.0 {
                    acc + 1
                } else {
                    acc
                }
            },
        )
        .sum();

    let total_b_wins = total_games - total_a_wins;

    let a_b_win_ratio = total_a_wins as f64 / total_games as f64;

    println!(
        "{} games, player A won {}, player B won {}",
        total_games, total_a_wins, total_b_wins
    );

    let confidence_interval = wilson_score(a_b_win_ratio, total_games as u64);
    println!(
        "p(a_wins) = {:.2} (p95 [{:.4}, {:.4}])",
        a_b_win_ratio, confidence_interval.0, confidence_interval.1
    );

    let average_length = turns.iter().copied().map(|i| i as f64).sum::<f64>() / total_games as f64;
    let (shortest_game, longest_game) = turns.iter().copied().minmax().into_option().unwrap();

    println!(
        "average game length: {:.1} ({}..{})",
        average_length, shortest_game, longest_game
    );

    let mut move_distribution = [[0; MAX_MOVES]; 2];

    for s in stats.iter() {
        for i in 0..MAX_MOVES {
            move_distribution[0][i] += s.move_distribution[0][i] as u32;
            move_distribution[1][i] += s.move_distribution[1][i] as u32;
        }
    }

    let choice_percentage_a = move_distribution[0][1..MAX_MOVES]
        .iter()
        .map(|&i| i as f64)
        .sum::<f64>()
        / move_distribution[0].iter().map(|&i| i as f64).sum::<f64>()
        * 100.0;

    let choice_percentage_b = move_distribution[1][1..MAX_MOVES]
        .iter()
        .map(|&i| i as f64)
        .sum::<f64>()
        / move_distribution[1].iter().map(|&i| i as f64).sum::<f64>()
        * 100.0;

    println!("move count distribution:");

    println!("{}", a.1.name());
    print_move_distribution_graph(move_distribution[0]);
    println!(
        "{:.1}% of turns had more than 1 option",
        choice_percentage_a
    );

    println!("{}", b.1.name());
    print_move_distribution_graph(move_distribution[1]);
    println!(
        "{:.1}% of turns had more than 1 option",
        choice_percentage_b
    );
}

fn compare_struggle_players(a: impl StrugglePlayer, b: impl StrugglePlayer, rounds: u32) {
    // It is a current unfortunate limitation of associated consts / const generics that we have to provde MAX_MOVES here :(
    compare_players_detailed::<4, StruggleGame<_, _>>(
        (PlayerColor::Red, a),
        (PlayerColor::Yellow, b),
        rounds,
    );
}

fn compare_twist_players(a: impl TwistPlayer, b: impl TwistPlayer, rounds: u32) {
    // It is a current unfortunate limitation of associated consts / const generics that we have to provde MAX_MOVES here :(
    compare_players_detailed::<25, TwistGame<_, _>>(
        (PlayerColor::Red, a),
        (PlayerColor::Yellow, b),
        rounds,
    );
}

pub fn main() {
    compare_struggle_players(RandomPlayer, RandomPlayer, 500_000);

    compare_twist_players(TwistRandomPlayer, TwistRandomPlayer, 500_000);
}

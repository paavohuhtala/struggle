use indicatif::ParallelProgressIterator;
use itertools::Itertools;
use plotters::prelude::*;
use rayon::prelude::*;
use struggle_core::{
    game::{play_game, CreateGame, IntoGameStats, NamedPlayer},
    games::{
        struggle::{
            players::{
                expectiminimax, worst_expectiminimax, RandomDietPlayer, RandomEaterPlayer,
                RandomPlayer, ScoreMovePlayer, StrugglePlayer, WorstScoreMovePlayer,
            },
            PlayerColor, StruggleGame,
        },
        twist::{
            players::{
                TwistDoSomethingPlayer, TwistPlayer, TwistRandomPlayer, TwistScoreBoardPlayer,
                TwistScoreBoardPlayerMaximizeLength, TwistScoreBoardPlayerWorst,
                TwistScoreMovePlayer,
            },
            TwistGame,
        },
    },
};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
    svg_path: &str,
) {
    println!("{} ({:?}) vs {} ({:?})", a.1.name(), a.0, b.1.name(), b.0);

    let start_time = std::time::Instant::now();

    let results = (0..rounds)
        .into_par_iter()
        .with_min_len(1000)
        .progress_count(rounds as u64)
        .map(|_| {
            let mut game = G::create_game(a.clone(), b.clone(), true);
            let winner = play_game(&mut game);
            (winner, game.into_stats().unwrap())
        })
        .collect::<Vec<_>>();

    let elapsed = start_time.elapsed();

    println!(
        "Finished {} rounds in {}.{:03}s ({} μs per round)",
        rounds,
        elapsed.as_secs(),
        elapsed.subsec_millis(),
        elapsed.as_micros() / rounds as u128
    );

    let drawing_area = SVGBackend::new(svg_path, (1500, 1500)).into_drawing_area();
    drawing_area.fill(&WHITE).unwrap();

    let (upper, lower) = drawing_area.split_vertically(750);

    let (lower_left, lower_right) = lower.split_horizontally(750);

    let total_games = results.len();
    let (winners, stats): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let turns = stats.iter().map(|stats| stats.turns as u32).collect_vec();
    let (&min_turns, &max_turns) = turns.iter().minmax().into_option().unwrap();
    let turn_counts = turns.iter().counts();
    let most_common_turn = turn_counts.values().copied().max().unwrap() as u32;

    let total_eats = stats
        .iter()
        .map(|s| s.pieces_eaten_by)
        .fold([0, 0], |acc, eats| [acc[0] + eats[0], acc[1] + eats[1]]);

    let average_eats_per_player = [
        total_eats[0] as f64 / total_games as f64,
        total_eats[1] as f64 / total_games as f64,
    ];

    println!(
        "average pieces eaten: {} vs {}",
        average_eats_per_player[0], average_eats_per_player[1]
    );

    let mut ctx = ChartBuilder::on(&upper)
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
        let bar = Rectangle::new(
            [(x0, 0), (x1, count as u32)],
            RGBColor(68, 63, 212).filled(),
        );
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

    draw_move_distribution_histogram(&move_distribution[0], lower_left, "A", &a.1.name());
    draw_move_distribution_histogram(&move_distribution[1], lower_right, "B", &b.1.name());

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

    println!(
        "{}: {:.1}% of turns had more than 1 option",
        a.1.name(),
        choice_percentage_a
    );

    println!(
        "{}: {:.1}% of turns had more than 1 option",
        b.1.name(),
        choice_percentage_b
    );
}

fn draw_move_distribution_histogram<const MAX_MOVES: usize>(
    distribution: &[u32; MAX_MOVES],
    drawing_area: DrawingArea<SVGBackend, plotters::coord::Shift>,
    player_id: &'static str,
    player_name: &str,
) {
    let total_moves = distribution.iter().copied().sum::<u32>();
    let most_common_number_of_moves = distribution.iter().copied().max().unwrap();

    let mut chart = ChartBuilder::on(&drawing_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .margin(4)
        .caption(
            format!("Player {} ({}) number of choices", player_name, player_id,),
            ("Source Sans Pro, sans-serif", 20),
        )
        .build_cartesian_2d(
            (0..MAX_MOVES).into_segmented(),
            0..((most_common_number_of_moves as f32 * 1.05) as u32),
        )
        .unwrap();

    chart
        .configure_mesh()
        .y_label_formatter(&|coord| format!("{:.1}%", (*coord as f32 / total_moves as f32) * 100.0))
        .draw()
        .unwrap();

    chart
        .draw_series((0..MAX_MOVES).map(|i| {
            let count = distribution[i];
            let x0 = SegmentValue::Exact(i);
            let x1 = SegmentValue::Exact(i + 1);
            let mut bar = Rectangle::new([(x0, 0), (x1, count as u32)], MAGENTA.filled());
            bar.set_margin(0, 0, 1, 1);
            bar
        }))
        .unwrap();
}

#[allow(dead_code)]
fn compare_struggle_players(a: impl StrugglePlayer, b: impl StrugglePlayer, rounds: u32) {
    // It is a current unfortunate limitation of associated consts / const generics that we have to provde MAX_MOVES here :(
    compare_players_detailed::<4, StruggleGame<_, _>>(
        (PlayerColor::Red, a),
        (PlayerColor::Yellow, b),
        rounds,
        "out/struggle.svg",
    );
}

fn compare_twist_players(a: impl TwistPlayer, b: impl TwistPlayer, rounds: u32, svg_path: &str) {
    // It is a current unfortunate limitation of associated consts / const generics that we have to provde MAX_MOVES here :(
    compare_players_detailed::<25, TwistGame<_, _>>(
        (PlayerColor::Red, a),
        (PlayerColor::Yellow, b),
        rounds,
        svg_path,
    );
}

pub fn main() {
    std::fs::create_dir_all("out").unwrap();

    compare_struggle_players(expectiminimax(0), RandomPlayer, 100_000);
    compare_struggle_players(expectiminimax(1), RandomPlayer, 100_000);
    compare_struggle_players(expectiminimax(2), RandomPlayer, 100_000);
    compare_struggle_players(expectiminimax(4), RandomPlayer, 10_00);
    //compare_struggle_players(expectiminimax(3), RandomPlayer, 10_000);

    /*compare_struggle_players(expectiminimax(0), expectiminimax(0), 10_000);
    compare_struggle_players(expectiminimax(0), expectiminimax(1), 10_000);
    compare_struggle_players(expectiminimax(0), expectiminimax(2), 10_000);
    compare_struggle_players(expectiminimax(0), expectiminimax(3), 10_000);*/

    //compare_struggle_players(expectiminimax(1), expectiminimax(1), 200_000);

    /*compare_twist_players(
        TwistScoreBoardPlayerMaximizeLength,
        TwistDoSomethingPlayer,
        200_000,
        "maximize_length_vs_something.svg",
    );*/

    /*compare_twist_players(
        TwistDoSomethingPlayer,
        TwistRandomPlayer,
        200_000,
        "out/something_vs_random.svg",
    );

    compare_twist_players(
        TwistScoreMovePlayer,
        TwistRandomPlayer,
        200_000,
        "out/score_move_vs_random.svg",
    );

    compare_twist_players(
        TwistScoreBoardPlayer,
        TwistRandomPlayer,
        200_000,
        "out/score_board_vs_random.svg",
    );

    compare_twist_players(
        TwistScoreBoardPlayer,
        TwistScoreMovePlayer,
        200_000,
        "out/score_board_vs_score_move.svg",
    );

    compare_twist_players(
        TwistScoreBoardPlayer,
        TwistScoreBoardPlayerWorst,
        200_000,
        "out/score_board_vs_score_move.svg",
    );*/
}

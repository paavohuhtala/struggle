use std::collections::HashMap;

use rayon::prelude::*;
use struggle_core::{
    play_game,
    players::{
        expectiminimax, maximize_options_expectiminimax, minimize_options_expectiminimax,
        participatory_expectiminimax, worst_expectiminimax, DilutedPlayer, RandomDietPlayer,
        RandomEaterPlayer, RandomPlayer, StrugglePlayer,
    },
    struggle::Player,
};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
            let player_a = a.1.clone();
            let player_b = b.1.clone();
            play_game((a_color, player_a), (b_color, player_b), false)
        })
        .filter(|res| res.winner == a_color)
        .count();

    games_won_by_a as f64 / rounds as f64
}

const TOTAL_GAMES: u32 = 500_000;

macro_rules! run_games {
    ($player_l: expr, [$($player_r: expr),*], $out: expr) => {
        {
            let output: &mut HashMap<String, HashMap<String, f64>> = $out;
            let player_a = $player_l;
            let name = player_a.name();

            $(
                let player_b = $player_r;
                let name_b = player_b.name();
                let p_a = compare_players((Player::Red, player_a.clone()), (Player::Yellow, player_b.clone()), TOTAL_GAMES);
                println!("{} vs {}: {}", name, name_b, p_a);
                output.entry(name.to_string()).or_insert_with(HashMap::new).insert(name_b.to_string(), p_a);

                if name != name_b {
                    output.entry(name_b.to_string()).or_insert_with(HashMap::new).insert(name.to_string(), 1.0 - p_a);
                }
            )*
        }
    };
}

pub fn main() {
    let mut results = HashMap::new();
    let mut writer = csv::Writer::from_path("./results.csv").unwrap();

    run_games!(
        RandomPlayer,
        [
            RandomPlayer,
            RandomEaterPlayer,
            RandomDietPlayer,
            expectiminimax(1),
            worst_expectiminimax(1),
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        RandomEaterPlayer,
        [
            RandomEaterPlayer,
            RandomDietPlayer,
            expectiminimax(1),
            worst_expectiminimax(1),
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        RandomDietPlayer,
        [
            RandomDietPlayer,
            expectiminimax(1),
            worst_expectiminimax(1),
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        expectiminimax(1),
        [
            expectiminimax(1),
            worst_expectiminimax(1),
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        worst_expectiminimax(1),
        [
            worst_expectiminimax(1),
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        participatory_expectiminimax(1),
        [
            participatory_expectiminimax(1),
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        maximize_options_expectiminimax(1),
        [
            maximize_options_expectiminimax(1),
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        minimize_options_expectiminimax(1),
        [
            minimize_options_expectiminimax(1),
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        DilutedPlayer(expectiminimax(1), 0.5),
        [
            DilutedPlayer(expectiminimax(1), 0.5),
            DilutedPlayer(expectiminimax(1), 0.1)
        ],
        &mut results
    );

    run_games!(
        DilutedPlayer(expectiminimax(1), 0.1),
        [DilutedPlayer(expectiminimax(1), 0.1)],
        &mut results
    );

    let headers = vec![
        "Random",
        "RandomDiet",
        "RandomEater",
        "Expectiminimax(1)",
        "WorstExpectiminimax(1)",
        "ParticipatoryExpectiminimax(1)",
        "MaximizeOptionsExpectiminimax(1)",
        "MinimizeOptionsExpectiminimax(1)",
        "Expectiminimax(1) 50%",
        "Expectiminimax(1) 10%",
    ];
    writer.write_field("").unwrap();
    writer.write_record(&headers).unwrap();

    for key in &headers {
        let mut row = vec![key.to_string()];

        for key_b in &headers {
            let value = *results[*key].get(*key_b).unwrap_or(&0.0);
            row.push(format!("{:.2}", value));
        }
        writer.write_record(&row).unwrap();
    }
}

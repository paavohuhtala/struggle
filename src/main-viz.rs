use struggle_core::{
    game::{AiStrugglePlayer, RaceGame, StruggleGame, TurnResult},
    players::maximize_length_expectiminimax,
    struggle::{Board, PlayerColor, COLORS},
};

use ::rand::prelude::*;
use macroquad::prelude::*;

pub const WIDTH: usize = 1000;
pub const HEIGHT: usize = 1000;
pub const OUTER_RADIUS: f32 = 500.0;
pub const INNER_RADIUS: f32 = 440.0;
pub const PIECE_RADIUS: f32 = 30.0;
pub const GOAL_SEPARATION: f32 = 70.0;

pub fn player_to_color(player: PlayerColor) -> Color {
    match player {
        PlayerColor::Red => RED,
        PlayerColor::Blue => BLUE,
        PlayerColor::Yellow => YELLOW,
        PlayerColor::Green => GREEN,
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Struggle!".to_string(),
        window_resizable: false,
        window_width: WIDTH as i32,
        window_height: HEIGHT as i32,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let sector = (360.0 / Board::TILES as f32).to_radians();

    let center_x = WIDTH as f32 / 2.0;
    let center_y = HEIGHT as f32 / 2.0;

    let player_a = AiStrugglePlayer::new(PlayerColor::Red, maximize_length_expectiminimax(2));
    let player_b = AiStrugglePlayer::new(PlayerColor::Yellow, maximize_length_expectiminimax(2));

    let mut rng = SmallRng::from_rng(::rand::thread_rng()).unwrap();

    let mut next_tick = 0.0;

    let mut winner = None;

    let mut red_score = 0.0;
    let mut yellow_score = 0.0;

    let mut last_die = 0;
    let mut last_die_player = PlayerColor::Red;

    let mut game = StruggleGame::new(player_a.clone(), player_b.clone(), false);

    loop {
        let time = get_time();

        if time > next_tick && winner.is_none() {
            match game.play_turn(&mut rng) {
                TurnResult::PlayAgain => {}
                TurnResult::PassTo(player) => {
                    game.set_current_player(player);
                }
                TurnResult::EndGame {
                    winner: game_winner,
                } => {
                    winner = Some(game_winner);
                }
            }

            next_tick = time + 0.2;
        }

        if is_key_pressed(KeyCode::R) {
            game = StruggleGame::new(player_a.clone(), player_b.clone(), false);
            winner = None;
            red_score = 0.0;
            yellow_score = 0.0;
            last_die = 0;
            last_die_player = PlayerColor::Red;
        }

        clear_background(BLACK);

        draw_poly(center_x, center_y, 64, OUTER_RADIUS, 0.0, GRAY);

        draw_text(
            &last_die.to_string(),
            center_x,
            center_y,
            40.0,
            player_to_color(last_die_player),
        );

        for (i, tile) in game.board().tiles.iter().enumerate() {
            let relative_rad = i as f32 * sector;
            let x = center_x + INNER_RADIUS * relative_rad.cos();
            let y = center_y + INNER_RADIUS * relative_rad.sin();

            let is_home_base = i % 7 == 0;

            let game_color = if is_home_base {
                Some(COLORS[i / 7])
            } else {
                None
            };

            let base_color = game_color.map(player_to_color).unwrap_or(WHITE);

            match tile {
                None => {
                    draw_circle_lines(x, y, PIECE_RADIUS, 2.0, base_color);
                }
                Some(player) => {
                    draw_circle(x, y, PIECE_RADIUS, player_to_color(*player));
                }
            }

            if is_home_base {
                let side = i / 7;

                let mid = relative_rad - (sector / 2.0);
                let cos = mid.cos();
                let sin = mid.sin();

                let goals = game.board().goals[side as usize];

                // goals
                for (i, cell) in goals.iter().enumerate() {
                    let i = i + 1;
                    let distance = INNER_RADIUS - (i as f32 * GOAL_SEPARATION);
                    let x = center_x + distance * cos;
                    let y = center_y + distance * sin;

                    if cell.is_some() {
                        draw_circle(x, y, PIECE_RADIUS, base_color);
                    } else {
                        draw_circle_lines(x, y, PIECE_RADIUS, 2.0, base_color);
                    }

                    let text = match i {
                        1 => "1",
                        2 => "2",
                        3 => "3",
                        4 => "4",
                        _ => unreachable!(),
                    };
                    draw_text(text, x, y, 30.0, BLACK);
                }

                let home_base = &game.board().home_bases[side];

                // home base
                for i in 0..4 {
                    let distance = INNER_RADIUS + 45.0;

                    let center = mid + i as f32 * 5.0f32.to_radians();
                    let x = center_x + distance * center.cos();
                    let y = center_y + distance * center.sin();

                    if home_base.pieces_waiting > i {
                        draw_circle(x, y, 8.0, base_color);
                    } else {
                        draw_circle_lines(x, y, 8.0, 2.0, base_color);
                    }
                }
            }
        }

        draw_text(&format!("{}", red_score), 16.0, 30.0, 40.0, RED);
        draw_text(&format!("{}", yellow_score), 16.0, 70.0, 40.0, YELLOW);

        next_frame().await
    }
}

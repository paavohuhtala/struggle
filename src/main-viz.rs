use struggle_core::{
    players::{
        basic_heuristic, one_at_a_time_expectimax, random_expectimax, GameContext, StrugglePlayer,
    },
    struggle::{Board, Player, COLORS},
};

use ::rand::prelude::*;
use macroquad::prelude::*;

pub const WIDTH: usize = 1000;
pub const HEIGHT: usize = 1000;
pub const OUTER_RADIUS: f32 = 500.0;
pub const INNER_RADIUS: f32 = 440.0;
pub const PIECE_RADIUS: f32 = 30.0;
pub const GOAL_SEPARATION: f32 = 70.0;

pub fn player_to_color(player: Player) -> Color {
    match player {
        Player::Red => RED,
        Player::Blue => BLUE,
        Player::Yellow => YELLOW,
        Player::Green => GREEN,
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
    let mut board = Board::new();
    let sector = (360.0 / board.tiles.len() as f32).to_radians();

    let center_x = WIDTH as f32 / 2.0;
    let center_y = HEIGHT as f32 / 2.0;

    let player_a = (Player::Red, one_at_a_time_expectimax(2));
    let player_b = (Player::Yellow, random_expectimax());

    let player_a_color = player_a.0;
    let player_b_color = player_b.0;

    let mut current_player = player_a;
    let mut other_player = player_b;

    let mut rng = SmallRng::from_rng(::rand::thread_rng()).unwrap();

    let mut next_tick = 0.0;

    let mut winnage = false;

    let mut red_score = 0.0;
    let mut yellow_score = 0.0;

    let mut last_die = 0;
    let mut last_die_player = Player::Red;

    loop {
        let time = get_time();

        if time > next_tick && !winnage {
            let dice = rng.gen_range(1..=6);
            last_die = dice;
            last_die_player = current_player.0;

            let moves = board.get_moves(dice, current_player.0, other_player.0);

            let ctx = GameContext {
                current_player: current_player.0,
                other_player: other_player.0,
                dice,
            };

            let mov = current_player
                .1
                .select_move(&ctx, &board, &moves, &mut rng)
                .clone();
            board.perform_move(current_player.0, &mov);

            if let Some(_) = board.get_winner() {
                winnage = true;
            }

            if dice != 6 {
                std::mem::swap(&mut current_player, &mut other_player);
            }

            red_score = basic_heuristic(&board, player_a_color, player_b_color);
            yellow_score = basic_heuristic(&board, player_b_color, player_a_color);

            next_tick = time + 0.2;
        }

        if is_key_pressed(KeyCode::R) {
            board = Board::new();
            winnage = false;
            red_score = 0.0;
            yellow_score = 0.0;
            last_die = 0;
            last_die_player = Player::Red;
        }

        clear_background(BLACK);

        draw_circle(center_x, center_y, OUTER_RADIUS, GRAY);

        draw_text(
            &last_die.to_string(),
            center_x,
            center_y,
            40.0,
            player_to_color(last_die_player),
        );

        for (i, tile) in board.tiles.iter().enumerate() {
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

                let goals = board.goals[side as usize];

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
                    draw_text(text, x, y, 30.0, WHITE);
                }

                let home_base = &board.home_bases[side];

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

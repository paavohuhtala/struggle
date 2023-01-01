use arrayvec::ArrayVec;

use crate::games::struggle::{board::PiecePosition, PlayerColor};

use super::board::{
    ActionDie, ActionDieMove, DieResult, MoveFrom, NumberDieMove, SpinSection, TwistBoard,
    TwistMove, TwistMoveVec,
};

fn spin_is_nop(board: &TwistBoard, spin_section: SpinSection) -> bool {
    let section = board.get_spin_section(spin_section);

    let mut rev_section = section.clone();
    rev_section.reverse();

    section == &rev_section
}

fn create_move_to_pos(
    board: &TwistBoard,
    player: PlayerColor,
    from: MoveFrom,
    to_pos: u8,
) -> Option<NumberDieMove> {
    let target_tile = board.tiles[to_pos as usize];

    // Do not allowing adding a new piece if home is empty
    if from == MoveFrom::Home && !board.home_bases[player as usize].can_add_piece() {
        return None;
    }

    match target_tile {
        None => Some(NumberDieMove::MovePiece {
            from,
            to: to_pos,
            eats: false,
        }),
        Some(target_player) if target_player != player => Some(NumberDieMove::MovePiece {
            from,
            to: to_pos,
            eats: true,
        }),
        Some(_) => None,
    }
}

pub fn get_twist_moves(
    board: &TwistBoard,
    dice: DieResult,
    player: PlayerColor,
    enemy: PlayerColor,
) -> TwistMoveVec {
    let mut number_die_moves = <ArrayVec<NumberDieMove, 5>>::new_const();
    let mut action_die_moves = <ArrayVec<ActionDieMove, 5>>::new_const();

    let home_base = &board.home_bases[player as usize];
    let (player_pieces, _) = board.get_pieces(player, enemy);
    let player_start = TwistBoard::STARTS[player as usize];

    let pieces_on_board = player_pieces
        .iter()
        .filter(|p| matches!(p, PiecePosition::Board(_)))
        .count();

    // New rule: you can move a piece from home to start with any number _if_ you only have pieces in home AND/OR goal area
    if ((pieces_on_board == 0) || (dice.number == 6)) && home_base.pieces_waiting > 0 {
        if let Some(mov) = create_move_to_pos(board, player, MoveFrom::Home, player_start) {
            number_die_moves.push(mov);
        }
    }

    for piece in player_pieces {
        match piece {
            PiecePosition::Goal(_) => {
                // In this variant pieces in goal can't move (as far as I know)
            }
            PiecePosition::Board(current_pos) => {
                let current_pos = *current_pos;
                let new_pos = (current_pos + dice.number) % TwistBoard::TILES as u8;

                let distance_to_goal = board.distance_to_goal(player, current_pos);

                if dice.number >= distance_to_goal {
                    let movement_after_goal_entrance = (dice.number - distance_to_goal).min(3);

                    // If we can only reach the entrance (the non-moving part of the goal), check if there is a piece there
                    if movement_after_goal_entrance == 0 {
                        if let Some(mov) =
                            create_move_to_pos(board, player, MoveFrom::Board(current_pos), new_pos)
                        {
                            number_die_moves.push(mov);
                        }
                    } else {
                        for i in (1..=movement_after_goal_entrance).rev() {
                            // Goal coordinates start from 0, so we need to subtract 1
                            let i = i - 1;

                            let is_tile_free = board.goals[player as usize][i as usize].is_none();

                            if is_tile_free {
                                number_die_moves.push(NumberDieMove::MoveToGoal {
                                    from_board: current_pos,
                                    to_goal: i,
                                });

                                // Break after first free tile
                                // In our variant the player can only move to the last ("deepest") available goal tile
                                break;
                            }
                        }
                    }
                } else {
                    // We can't reach the goal entrance or any of the goal tiles, so do a normal move
                    if let Some(mov) =
                        create_move_to_pos(board, player, MoveFrom::Board(current_pos), new_pos)
                    {
                        number_die_moves.push(mov);
                    }
                }
            }
        }
    }

    // Movement die can alwys be ignored
    number_die_moves.push(NumberDieMove::DoNothing);

    match dice.action {
        ActionDie::SpinSection => {
            for section in SpinSection::ALL {
                let spin_is_nop = spin_is_nop(board, section);

                if !spin_is_nop {
                    action_die_moves.push(ActionDieMove::SpinSection(section));
                }
            }
        }
        ActionDie::RotateBoard => {
            action_die_moves.push(ActionDieMove::RotateBoard);
        }
        ActionDie::DoNothing => {}
    }

    // DoNothing is always a valid move; the action die can be ignored
    action_die_moves.push(ActionDieMove::DoNothing);

    // Create a cartesian product of all possible moves

    let mut moves = TwistMoveVec::new();

    for number_die_move in number_die_moves {
        for action_die_move in &action_die_moves {
            moves.push(TwistMove(number_die_move.clone(), action_die_move.clone()))
        }
    }

    moves
}

#[cfg(test)]
mod get_moves_tests {
    use assert_unordered::assert_eq_unordered_sort;

    use crate::{
        games::twist::{board::TwistRotation, get_moves::get_twist_moves},
        tinyvec_util::TinyVecExt,
    };

    use super::*;

    const P1: PlayerColor = PlayerColor::Red;
    const P2: PlayerColor = PlayerColor::Yellow;

    const P1_GOAL_ENTRANCE: u8 = TwistBoard::get_goal_entrance(TwistRotation::Initial, P1);

    #[test]
    fn test_add_new_piece() {
        let mut board = TwistBoard::new((P1, P2));

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 1,
                action: ActionDie::DoNothing,
            },
            P1,
            P2,
        );
        let moves = moves.into_vec();

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Home,
                    to: TwistBoard::RED_START,
                    eats: false,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[TwistBoard::RED_START as usize], Some(P1));
        assert_eq!(board.home_bases[P1 as usize].pieces_waiting, 3);
    }

    #[test]
    fn add_new_piece_eats() {
        let mut board = TwistBoard::new((P1, P2));

        board.tiles[TwistBoard::RED_START as usize] = Some(P2);
        board.home_bases[P2 as usize].pieces_waiting = 3;
        board.update_piece_cache();

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 1,
                action: ActionDie::DoNothing,
            },
            P1,
            P2,
        )
        .into_vec();

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Home,
                    to: TwistBoard::RED_START,
                    eats: true,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[TwistBoard::RED_START as usize], Some(P1));
        assert_eq!(board.home_bases[P1 as usize].pieces_waiting, 3);
        assert_eq!(board.home_bases[P2 as usize].pieces_waiting, 4);
    }

    #[test]
    fn add_new_piece_own_already_in_start() {
        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            board.tiles[TwistBoard::RED_START as usize] = Some(P1);
            board.home_bases[P1 as usize].pieces_waiting = 3;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 6,
                action: ActionDie::DoNothing,
            },
            P1,
            P2,
        )
        .into_vec();

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Board(TwistBoard::RED_START),
                    to: TwistBoard::RED_START + 6,
                    eats: false,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);
    }

    #[test]
    fn move_wrap_around() {
        let mut board = TwistBoard::new((P2, P1));
        board.update(|board| {
            board.tiles[31] = Some(P2);
            board.home_bases[P2 as usize].pieces_waiting = 3;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 2,
                action: ActionDie::DoNothing,
            },
            P2,
            P1,
        )
        .into_vec();

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Board(31),
                    to: 1,
                    eats: false,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P2, &moves[0]);

        assert_eq!(board.tiles[1], Some(P2));
        assert_eq!(board.tiles[31], None);
        assert_eq!(board.home_bases[P2 as usize].pieces_waiting, 3);
    }

    fn get_approach_goal_state(
        distance: u8,
        die: u8,
        rotation: TwistRotation,
    ) -> (TwistBoard, Vec<TwistMove>) {
        let goal_entrance = TwistBoard::get_goal_entrance(rotation, P1);

        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            board.tiles[(goal_entrance - distance) as usize] = Some(P1);
            board.rotation = rotation;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: die,
                action: ActionDie::DoNothing,
            },
            P1,
            P2,
        )
        .into_vec();

        (board, moves)
    }

    #[test]
    fn approach_goal_1() {
        let (mut board, moves) = get_approach_goal_state(1, 1, TwistRotation::Initial);

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Board(P1_GOAL_ENTRANCE - 1),
                    to: P1_GOAL_ENTRANCE,
                    eats: false,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[P1_GOAL_ENTRANCE as usize], Some(P1));
        assert_eq!(board.tiles[(P1_GOAL_ENTRANCE - 1) as usize], None);
    }

    #[test]
    fn enter_goal_1() {
        let (mut board, moves) = get_approach_goal_state(0, 1, TwistRotation::Initial);

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MoveToGoal {
                    from_board: (P1_GOAL_ENTRANCE),
                    to_goal: 0,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[P1_GOAL_ENTRANCE as usize], None);
        assert_eq!(&board.goals[P1 as usize], &[Some(P1), None, None]);
    }

    #[test]
    fn enter_goal_6() {
        let (mut board, moves) = get_approach_goal_state(0, 6, TwistRotation::Initial);

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MovePiece {
                    from: MoveFrom::Home,
                    to: TwistBoard::RED_START,
                    eats: false,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove(
                NumberDieMove::MoveToGoal {
                    from_board: (P1_GOAL_ENTRANCE),
                    to_goal: 2,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[1]);

        assert_eq!(board.tiles[P1_GOAL_ENTRANCE as usize], None);
        assert_eq!(&board.goals[P1 as usize], &[None, None, Some(P1)]);
    }

    #[test]
    fn enter_goal_1_rotated() {
        let (mut board, moves) = get_approach_goal_state(1, 4, TwistRotation::Ccw180);

        // When board is rotated 180 degrees, red's goal entrance is in the same position as yellow's in the initial rotation
        let yellow_entrance =
            TwistBoard::get_goal_entrance(TwistRotation::Initial, PlayerColor::Yellow);

        let expected_moves = vec![
            TwistMove(
                NumberDieMove::MoveToGoal {
                    from_board: (yellow_entrance - 1),
                    to_goal: 2,
                },
                ActionDieMove::DoNothing,
            ),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[(yellow_entrance - 1) as usize], None);
        assert_eq!(&board.goals[P1 as usize], &[None, None, Some(P1)]);
    }

    #[test]
    fn rotate_board_1() {
        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            board.tiles[TwistBoard::RED_START as usize] = Some(P1);
            board.home_bases[P1 as usize].pieces_waiting = 3;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 1,
                action: ActionDie::RotateBoard,
            },
            P1,
            P2,
        )
        .into_vec();

        let number_move = NumberDieMove::MovePiece {
            from: MoveFrom::Board(TwistBoard::RED_START),
            to: TwistBoard::RED_START + 1,
            eats: false,
        };

        let action_move = ActionDieMove::RotateBoard;

        let expected_moves = vec![
            TwistMove(number_move.clone(), action_move.clone()),
            TwistMove(number_move, ActionDieMove::DoNothing),
            TwistMove(NumberDieMove::DoNothing, action_move),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        assert_eq!(board.tiles[TwistBoard::RED_START as usize], None);
        assert_eq!(board.tiles[(TwistBoard::RED_START + 1) as usize], Some(P1));
        assert_eq!(board.rotation, TwistRotation::Ccw90);
    }

    #[test]
    fn spin_section_all_useless() {
        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            board.tiles[TwistBoard::RED_START as usize] = Some(P1);
            board.home_bases[P1 as usize].pieces_waiting = 3;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 1,
                action: ActionDie::SpinSection,
            },
            P1,
            P2,
        )
        .into_vec();

        let number_move = NumberDieMove::MovePiece {
            from: MoveFrom::Board(TwistBoard::RED_START),
            to: TwistBoard::RED_START + 1,
            eats: false,
        };

        // Since there are no pieces on spin sections, we not generate moves for it
        let expected_moves = vec![
            TwistMove(number_move.clone(), ActionDieMove::DoNothing),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);
    }

    #[test]
    fn spin_section_basic() {
        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            let spin_section = board.get_spin_section_mut(SpinSection::RedToBlue);
            spin_section[0] = Some(P1);
            spin_section[4] = Some(P2);

            board.home_bases[P1 as usize].pieces_waiting = 3;
            board.home_bases[P2 as usize].pieces_waiting = 3;
        });

        let moves = get_twist_moves(
            &board,
            DieResult {
                number: 1,
                action: ActionDie::SpinSection,
            },
            P1,
            P2,
        )
        .into_vec();

        let spin_section_range = TwistBoard::get_spin_section_range(SpinSection::RedToBlue);

        let number_move = NumberDieMove::MovePiece {
            from: MoveFrom::Board(spin_section_range.start as u8),
            to: (spin_section_range.start + 1) as u8,
            eats: false,
        };

        let action_move = ActionDieMove::SpinSection(SpinSection::RedToBlue);

        let expected_moves = vec![
            TwistMove(number_move.clone(), action_move.clone()),
            TwistMove(number_move, ActionDieMove::DoNothing),
            TwistMove(NumberDieMove::DoNothing, action_move),
            TwistMove::default(),
        ];

        assert_eq_unordered_sort!(&moves, &expected_moves);

        board.perform_move(P1, &moves[0]);

        // Red's piece went 1 forward but then spin section was rotated 180 degrees
        assert_eq!(
            board.get_spin_section(SpinSection::RedToBlue).as_slice(),
            &[Some(P2), None, None, Some(P1), None]
        );
    }
}

#[cfg(test)]
mod spin_is_nop_tests {
    use super::*;

    const P1: PlayerColor = PlayerColor::Red;
    const P2: PlayerColor = PlayerColor::Yellow;

    fn test_spin_section(initial: [Option<PlayerColor>; 5]) -> bool {
        let mut board = TwistBoard::new((P1, P2));
        board.update(|board| {
            let spin_section = board.get_spin_section_mut(SpinSection::RedToBlue);
            spin_section.copy_from_slice(&initial);
        });

        spin_is_nop(&board, SpinSection::RedToBlue)
    }

    #[test]
    fn test_spin_is_nop() {
        // Examples of useless spins
        assert!(test_spin_section([None, None, None, None, None]));
        assert!(test_spin_section([None, None, Some(P1), None, None]));
        assert!(test_spin_section([Some(P1), None, None, None, Some(P1)]));
        assert!(test_spin_section([
            Some(P1),
            None,
            Some(P2),
            None,
            Some(P1)
        ]));

        // Examples of useful spins
        assert!(!test_spin_section([Some(P1), None, None, None, None]));
        assert!(!test_spin_section([Some(P1), None, Some(P2), None, None]));
        assert!(!test_spin_section([Some(P1), None, None, None, Some(P2)]));
    }
}

use crate::{
    a_star::a_star,
    data_model::{
        Direction, Game, MovePiece, PIECE_GRID_HEIGHT, PiecePosition, Player, PlayerMove,
        WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, Walls,
    },
};

pub fn execute_move_unchecked(game: &Game, m: &PlayerMove) -> Game {
    let mut next = game.clone();
    execute_move_unchecked_inplace(&mut next, m);
    next
}

fn execute_move_unchecked_inplace(game: &mut Game, player_move: &PlayerMove) {
    let player = game.player;

    match player_move {
        PlayerMove::PlaceWall {
            orientation,
            position,
        } => {
            game.board.walls.0[position.x][position.y] = Some(*orientation);
            game.walls_left[player.as_index()] -= 1;
        }
        PlayerMove::MovePiece(move_piece) => {
            let new_position = new_position_after_move_piece_unchecked(
                game.board.player_position(player),
                move_piece,
                game.board.player_position(player.opponent()),
            );
            game.board.player_positions[player.as_index()] = new_position;
        }
    }
    game.player = player.opponent();
}

pub fn is_move_piece_legal(game: &Game, move_piece: &MovePiece) -> bool {
    is_move_piece_legal_with_players_at_positions(
        &game.board.walls,
        game.board.player_position(game.player),
        game.board.player_position(game.player.opponent()),
        move_piece,
    )
}
pub fn is_move_piece_legal_with_players_at_positions(
    walls: &Walls,
    player_position: &PiecePosition,
    opponent_position: &PiecePosition,
    move_piece: &MovePiece,
) -> bool {
    if is_move_direction_legal_with_player_at_position(
        walls,
        player_position,
        &move_piece.direction,
    ) {
        let new_position =
            new_position_after_direction_unchecked(player_position, move_piece.direction);
        if new_position == *opponent_position {
            is_move_direction_legal_with_player_at_position(
                walls,
                &new_position,
                &move_piece.direction_on_collision,
            )
        } else {
            true
        }
    } else {
        false
    }
}

pub fn is_move_direction_legal_with_player_at_position(
    walls: &Walls,
    player_position: &PiecePosition,
    direction: &Direction,
) -> bool {
    match direction {
        Direction::Up => {
            player_position.y > 0
                && !walls.wall_at(
                    WallOrientation::Horizontal,
                    player_position.x as isize - 1,
                    player_position.y as isize - 1,
                )
                && !walls.wall_at(
                    WallOrientation::Horizontal,
                    player_position.x as isize,
                    player_position.y as isize - 1,
                )
        }
        Direction::Down => {
            player_position.y < PIECE_GRID_HEIGHT - 1
                && !walls.wall_at(
                    WallOrientation::Horizontal,
                    player_position.x as isize - 1,
                    player_position.y as isize,
                )
                && !walls.wall_at(
                    WallOrientation::Horizontal,
                    player_position.x as isize,
                    player_position.y as isize,
                )
        }
        Direction::Left => {
            player_position.x > 0
                && !walls.wall_at(
                    WallOrientation::Vertical,
                    player_position.x as isize - 1,
                    player_position.y as isize,
                )
                && !walls.wall_at(
                    WallOrientation::Vertical,
                    player_position.x as isize - 1,
                    player_position.y as isize - 1,
                )
        }
        Direction::Right => {
            player_position.x < PIECE_GRID_HEIGHT - 1
                && !walls.wall_at(
                    WallOrientation::Vertical,
                    player_position.x as isize,
                    player_position.y as isize,
                )
                && !walls.wall_at(
                    WallOrientation::Vertical,
                    player_position.x as isize,
                    player_position.y as isize - 1,
                )
        }
    }
}

pub fn room_for_wall_placement(
    walls: &Walls,
    orientation: WallOrientation,
    x: isize,
    y: isize,
) -> bool {
    x >= 0
        && y >= 0
        && x < WALL_GRID_WIDTH as isize
        && y < WALL_GRID_HEIGHT as isize
        && walls.0[x as usize][y as usize].is_none()
        && match orientation {
            WallOrientation::Horizontal => [(-1, 0), (1, 0)],
            WallOrientation::Vertical => [(0, -1), (0, 1)],
        }
        .iter()
        .all(|(dx, dy)| !walls.wall_at(orientation, x + dx, y + dy))
}

pub fn is_move_legal(game: &Game, player_move: &PlayerMove) -> bool {
    match player_move {
        PlayerMove::MovePiece(move_piece) => is_move_piece_legal_with_players_at_positions(
            &game.board.walls,
            game.board.player_position(game.player),
            game.board.player_position(game.player.opponent()),
            move_piece,
        ),
        PlayerMove::PlaceWall {
            orientation,
            position,
        } => {
            let blocks_path = |player_to_block_check: Player| {
                let next_game_state = execute_move_unchecked(
                    game,
                    &PlayerMove::PlaceWall {
                        orientation: *orientation,
                        position: position.clone(),
                    },
                );
                a_star(&next_game_state.board, player_to_block_check).is_none()
            };
            game.walls_left[game.player.as_index()] > 0
                && room_for_wall_placement(
                    &game.board.walls,
                    *orientation,
                    position.x as isize,
                    position.y as isize,
                )
                && !blocks_path(game.player)
                && !blocks_path(game.player.opponent())
        }
    }
}

pub fn new_position_after_direction_unchecked(
    player_position: &PiecePosition,
    direction: Direction,
) -> PiecePosition {
    let (dx, dy) = direction.to_offset();
    PiecePosition::new(
        (player_position.x as isize + dx) as usize,
        (player_position.y as isize + dy) as usize,
    )
}

pub fn new_position_after_move_piece_unchecked(
    player_position: &PiecePosition,
    move_piece: &MovePiece,
    opponent_position: &PiecePosition,
) -> PiecePosition {
    let new_position =
        new_position_after_direction_unchecked(player_position, move_piece.direction);
    if opponent_position == &new_position {
        new_position_after_direction_unchecked(opponent_position, move_piece.direction_on_collision)
    } else {
        new_position
    }
}

pub fn all_move_piece_moves(
    player_position: &PiecePosition,
    opponent_position: &PiecePosition,
) -> impl Iterator<Item = MovePiece> {
    let x_diff = opponent_position.x as isize - player_position.x as isize;
    let y_diff = opponent_position.y as isize - player_position.y as isize;
    let jump_direction = match (x_diff, y_diff) {
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        (1, 0) => Some(Direction::Right),
        (-1, 0) => Some(Direction::Left),
        _ => None,
    };
    let jump_moves = jump_direction.map(|j| {
        Direction::iter()
            .filter(move |&d| d != j.opposite())
            .map(move |d| MovePiece {
                direction: j,
                direction_on_collision: d,
            })
    });
    let non_jump_moves = jump_direction.map(|j| {
        Direction::iter()
            .filter(move |&d| d != j)
            .map(|d| MovePiece {
                direction: d,
                direction_on_collision: d,
            })
    });
    let regular_moves = jump_direction
        .is_none()
        .then_some(Direction::iter().map(|d| MovePiece {
            direction: d,
            direction_on_collision: d,
        }));

    std::iter::empty()
        .chain(jump_moves.into_iter().flatten())
        .chain(non_jump_moves.into_iter().flatten())
        .chain(regular_moves.into_iter().flatten())
}

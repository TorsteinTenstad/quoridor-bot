use std::time::{Duration, SystemTime};

use crate::{
    a_star::a_star,
    a_star_to_opponent::a_star_to_opponent,
    data_model::{
        Direction, Game, MovePiece, PIECE_GRID_HEIGHT, Player, PlayerMove, TOTAL_WALLS,
        WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{
        execute_move_unchecked, is_move_piece_legal_with_player_at_position,
        room_for_wall_placement,
    },
    render_board,
    square_outline_iterator::SquareOutlineIterator,
};
pub const WHITE_LOSES_BLACK_WINS: isize = isize::MIN + 1;
pub const WHITE_WINS_BLACK_LOSES: isize = -WHITE_LOSES_BLACK_WINS;

pub fn heuristic_board_score(game: &Game) -> isize {
    let black_path = a_star(&game.board, Player::Black);
    let white_path = a_star(&game.board, Player::White);
    if white_path.is_none() {
        println!(
            "{:?} has no path in the following board:\n{}",
            Player::White,
            render_board::render_board(&game.board)
        );
    }
    let black_distance = black_path.unwrap().len() as isize;
    if black_distance == 0 {
        return WHITE_LOSES_BLACK_WINS;
    }
    let white_distance = white_path.unwrap().len() as isize;
    if white_distance == 0 {
        return WHITE_WINS_BLACK_LOSES;
    }
    let white_walls_left = game.walls_left[Player::White.as_index()] as isize;
    let black_walls_left = game.walls_left[Player::Black.as_index()] as isize;
    let distance_score = black_distance - white_distance;
    let wall_score = white_walls_left - black_walls_left;
    let total_walls_played = TOTAL_WALLS
        - game.walls_left[Player::White.as_index()]
        - game.walls_left[Player::Black.as_index()];
    let wall_progress = total_walls_played as f32 / TOTAL_WALLS as f32;
    let wall_value = 0.75 - 0.5 * wall_progress;
    let (distance_priority, wall_priority) = (1, wall_value);

    let path_length_between_players = a_star_to_opponent(&game.board, game.player)
        .map(|v| v.len())
        .unwrap_or(usize::MAX);

    let side = 2.0 * game.board.player_position(game.player).y() as f32
        / (PIECE_GRID_HEIGHT - 1) as f32
        - 1.0;

    let side_component = -side * 20.0 / path_length_between_players as f32;

    distance_priority * distance_score
        + (wall_priority * wall_score as f32 + side_component) as isize
}

pub fn best_move_alpha_beta_iterative_deepening(
    game: &Game,
    player: Player,
    search_duration: Duration,
) -> (isize, Option<PlayerMove>, usize) {
    let start = SystemTime::now();
    let stop = || SystemTime::now().duration_since(start).unwrap() > search_duration;

    let mut best_move: Option<PlayerMove> = None;
    let mut depth = 1;
    loop {
        let (score, new_move) = alpha_beta(
            game,
            depth,
            WHITE_LOSES_BLACK_WINS,
            WHITE_WINS_BLACK_LOSES,
            player,
            best_move.clone(),
            Some(&stop),
        );
        best_move = new_move;
        if stop() {
            break (score, best_move, depth);
        }
        depth += 1;
    }
}
pub fn best_move_alpha_beta(
    game: &Game,
    player: Player,
    depth: usize,
) -> (isize, Option<PlayerMove>) {
    alpha_beta(
        game,
        depth,
        WHITE_LOSES_BLACK_WINS,
        WHITE_WINS_BLACK_LOSES,
        player,
        None,
        None,
    )
}

pub fn alpha_beta(
    game: &Game,
    depth: usize,
    alpha: isize,
    beta: isize,
    player: Player,
    search_first: Option<PlayerMove>,
    stop: Option<&dyn Fn() -> bool>,
) -> (isize, Option<PlayerMove>) {
    let heuristic_board_score = heuristic_board_score(game);
    if depth == 0
        || heuristic_board_score == WHITE_LOSES_BLACK_WINS
        || heuristic_board_score == WHITE_WINS_BLACK_LOSES
    {
        return (heuristic_board_score, None);
    }
    let mut alpha = alpha;
    let mut beta = beta;
    let mut best_move = None;
    let score = match player {
        Player::White => {
            let mut value = WHITE_LOSES_BLACK_WINS;
            for player_move in moves_ordered_by_heuristic_quality(game, player, search_first) {
                let mut child_game_state = game.clone();
                execute_move_unchecked(&mut child_game_state, player, &player_move);
                if a_star(&child_game_state.board, player).is_none()
                    || a_star(&child_game_state.board, player.opponent()).is_none()
                {
                    continue;
                }
                let (score, _) = alpha_beta(
                    &child_game_state,
                    depth - 1,
                    alpha,
                    beta,
                    player.opponent(),
                    None,
                    None,
                );
                if score > value || best_move.is_none() {
                    best_move = Some(player_move);
                }
                value = isize::max(value, score);
                if value >= beta {
                    break;
                }
                alpha = isize::max(alpha, value);
                if stop.is_some_and(|f| f()) {
                    break;
                }
            }
            value
        }
        Player::Black => {
            let mut value = WHITE_WINS_BLACK_LOSES;
            for player_move in moves_ordered_by_heuristic_quality(game, player, search_first) {
                let mut child_game_state = game.clone();
                execute_move_unchecked(&mut child_game_state, player, &player_move);
                if a_star(&child_game_state.board, player).is_none()
                    || a_star(&child_game_state.board, player.opponent()).is_none()
                {
                    continue;
                }
                let (score, _) = alpha_beta(
                    &child_game_state,
                    depth - 1,
                    alpha,
                    beta,
                    player.opponent(),
                    None,
                    None,
                );
                if score < value || best_move.is_none() {
                    best_move = Some(player_move);
                }
                value = isize::min(value, score);
                if value <= alpha {
                    break;
                }
                beta = isize::min(beta, value);
                if stop.is_some_and(|f| f()) {
                    break;
                }
            }
            value
        }
    };
    (score, best_move)
}

fn moves_ordered_by_heuristic_quality(
    game: &Game,
    player: Player,
    search_first: Option<PlayerMove>,
) -> Vec<PlayerMove> {
    let mut moves: Vec<PlayerMove> = Default::default();
    if let Some(search_first) = search_first {
        moves.push(search_first); // TODO: Could ensure that the code below does not also add this mode. Unclear if this is worth it.
    }
    let player_position = game.board.player_position(player);
    let opponent_position = game.board.player_position(player.opponent());
    let x_diff = opponent_position.x() as isize - player_position.x() as isize;
    let y_diff = opponent_position.y() as isize - player_position.y() as isize;

    let push_if_move_piece_is_legal =
        |moves: &mut Vec<PlayerMove>, direction: Direction, direction_on_collision: Direction| {
            let move_piece = MovePiece {
                direction,
                direction_on_collision,
            };
            if is_move_piece_legal_with_player_at_position(
                &game.board,
                player,
                player_position,
                &move_piece,
            ) {
                moves.push(PlayerMove::MovePiece(move_piece));
            }
        };

    if let Some(jump_direction) = match (x_diff, y_diff) {
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        (1, 0) => Some(Direction::Right),
        (-1, 0) => Some(Direction::Left),
        _ => None,
    } {
        for direction in Direction::iter() {
            push_if_move_piece_is_legal(&mut moves, jump_direction, direction);
        }
        for direction in Direction::iter().filter(|&d| d != jump_direction) {
            push_if_move_piece_is_legal(&mut moves, direction, Direction::Up);
        }
    } else {
        for direction in Direction::iter() {
            push_if_move_piece_is_legal(&mut moves, direction, Direction::Up);
        }
    }
    if game.walls_left[player.as_index()] > 0 {
        let origin = opponent_position;
        for i in 1.. {
            let top_left_x = origin.x() as isize - i as isize;
            let top_left_y = origin.y() as isize - i as isize;
            let side_length = 2 * i;
            let mut some_in_bounds = false;
            for (x, y) in SquareOutlineIterator::new(top_left_x, top_left_y, side_length) {
                let in_bounds = x >= 0
                    && y >= 0
                    && x < WALL_GRID_WIDTH as isize
                    && y < WALL_GRID_HEIGHT as isize;
                if !in_bounds {
                    continue;
                }
                some_in_bounds = true;
                for orientation in [WallOrientation::Horizontal, WallOrientation::Vertical] {
                    let player_move = PlayerMove::PlaceWall {
                        orientation,
                        position: WallPosition {
                            x: x as usize,
                            y: y as usize,
                        },
                    };
                    if room_for_wall_placement(&game.board, orientation, x, y) {
                        moves.push(player_move);
                    }
                }
            }
            if !some_in_bounds {
                break;
            }
        }
    }
    moves
}

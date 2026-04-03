use crate::{
    data_model::{
        Game, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{all_move_piece_moves, is_move_piece_legal, room_for_wall_placement},
    square_outline_iterator::SquareOutlineIterator,
};

pub fn moves_ordered_by_heuristic_quality(game: &Game) -> Vec<PlayerMove> {
    let player_position = game.board.player_position(game.player);
    let opponent_position = game.board.player_position(game.player.opponent());

    let mut moves: Vec<PlayerMove> = all_move_piece_moves(player_position, opponent_position)
        .filter(move |move_piece| is_move_piece_legal(game, move_piece))
        .map(PlayerMove::MovePiece)
        .collect();
    if game.walls_left[game.player.as_index()] > 0 {
        let origin = opponent_position;
        for i in 1.. {
            let top_left_x = origin.x as isize - i as isize;
            let top_left_y = origin.y as isize - i as isize;
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
                    if room_for_wall_placement(&game.board.walls, orientation, x, y) {
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

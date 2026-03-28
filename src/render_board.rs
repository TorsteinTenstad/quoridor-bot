use crate::data_model::{
    Board, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation,
};

pub fn render_board(board: &Board) -> String {
    let mut output = String::new();
    for y in 0..PIECE_GRID_HEIGHT {
        if y > 0 {
            output.push('\n');
        }
        let draw_vertical_wall = |x: usize| {
            let wall_above = x < WALL_GRID_WIDTH
                && y > 0
                && matches!(board.walls[x][y - 1], Some(WallOrientation::Vertical));
            let wall_below = x < WALL_GRID_WIDTH
                && y < WALL_GRID_HEIGHT
                && matches!(board.walls[x][y], Some(WallOrientation::Vertical));
            if wall_below || wall_above { '│' } else { ' ' }
        };
        for x in 0..PIECE_GRID_WIDTH {
            output.push_str(format!("┌───┐ {} ", draw_vertical_wall(x)).as_str());
        }
        output.push('\n');
        for x in 0..PIECE_GRID_WIDTH {
            let player_char =
                if board.player_positions[0].x == x && board.player_positions[0].y == y {
                    'W'
                } else if board.player_positions[1].x == x && board.player_positions[1].y == y {
                    'B'
                } else {
                    ' '
                };
            output.push_str(format!("│ {} │ {} ", player_char, draw_vertical_wall(x)).as_str());
        }
        output.push('\n');
        for x in 0..PIECE_GRID_WIDTH {
            output.push_str(format!("└───┘ {} ", draw_vertical_wall(x)).as_str());
        }
        if y < WALL_GRID_HEIGHT {
            output.push('\n');
            for x in 0..PIECE_GRID_WIDTH {
                let wall_right = y < WALL_GRID_WIDTH
                    && x < WALL_GRID_HEIGHT
                    && matches!(board.walls[x][y], Some(WallOrientation::Horizontal));
                let wall_left = y < WALL_GRID_WIDTH
                    && x > 0
                    && matches!(board.walls[x - 1][y], Some(WallOrientation::Horizontal));
                let vertical_wall = x < WALL_GRID_WIDTH
                    && y < WALL_GRID_HEIGHT
                    && matches!(board.walls[x][y], Some(WallOrientation::Vertical));
                let vertical_wall_char = if vertical_wall { '│' } else { ' ' };
                let write_indices = x < WALL_GRID_WIDTH && !vertical_wall;
                let (x_str, y_str) = if write_indices {
                    (x.to_string(), y.to_string())
                } else {
                    (" ".to_string(), " ".to_string())
                };
                if wall_right {
                    output.push_str("────────");
                } else if wall_left {
                    output.push_str(
                        format!("─────{}{}{}", x_str, vertical_wall_char, y_str,).as_str(),
                    );
                } else {
                    output.push_str(
                        format!("     {}{}{}", x_str, vertical_wall_char, y_str,).as_str(),
                    );
                }
            }
        }
    }
    output
}

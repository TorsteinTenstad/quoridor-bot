use std::fmt::Debug;

use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
    WallOrientation,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Unreachable,
    Goal,
    Left,
    Right,
    Up,
    Down,
}

impl Debug for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "⊗"),
            Self::Goal => write!(f, "⊙"),
            Self::Left => write!(f, "🠈"),
            Self::Right => write!(f, "🠊"),
            Self::Up => write!(f, "🠉"),
            Self::Down => write!(f, "🠋"),
        }
    }
}

pub struct Board {
    path: [[(Dir, u8); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.path {
            for square in row {
                let _ = write!(f, "{:3}{:?} ", square.1, square.0);
            }
            let _ = write!(f, "\n");
        }
        Ok(())
    }
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        let game = game.clone();

        let mut path = [[(Dir::Unreachable, 255); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut visited = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue = [(0 as i8, 0 as i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        let player_pos = game.board.player_position(game.player);

        {
            let y = match game.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                if x == player_pos.x && y == player_pos.y {
                    return Board { path };
                }

                path[y][x] = (Dir::Goal, 0);
                visited[y][x] = true;
                queue[queue_len] = (x as i8, y as i8);
                queue_len += 1;
            }
        }

        let mut i = 0;
        'queue: while i < queue_len {
            let (x, y) = queue[i];
            for (dx, dy) in board_neighbors(&game, x, y) {
                let nx = x + dx;
                let ny = y + dy;

                if visited[ny as usize][nx as usize] {
                    continue;
                }

                let dist = path[y as usize][x as usize].1 + 1;
                path[ny as usize][nx as usize] = match (dx, dy) {
                    (-1, _) => (Dir::Right, dist),
                    (1, _) => (Dir::Left, dist),
                    (_, -1) => (Dir::Down, dist),
                    (_, 1) => (Dir::Up, dist),
                    _ => unreachable!(),
                };

                if nx as usize == player_pos.x && y as usize == player_pos.y {
                    break 'queue;
                }

                visited[ny as usize][nx as usize] = true;
                queue[queue_len] = (nx, ny);
                queue_len += 1;
            }

            i += 1;
        }

        return Board { path };
    }
}

/// Returns iterator over (dx, dy) for valid neighbours on the board.
fn board_neighbors(game: &Game, x: i8, y: i8) -> impl Iterator<Item = (i8, i8)> {
    [(-1, 0), (1, 0), (0, -1), (0, 1)]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;

            if nx < 0 || nx >= PIECE_GRID_WIDTH as i8 || ny < 0 || ny >= PIECE_GRID_HEIGHT as i8 {
                // Invalid out of bounds move.
                return None;
            }

            if wall_blocks(game, x, y, dx, dy) {
                return None;
            }

            Some((dx, dy))
        })
}

fn wall_blocks(game: &Game, x: i8, y: i8, dx: i8, dy: i8) -> bool {
    let orientation = match (dx, dy) {
        (0, _) => WallOrientation::Horizontal,
        (_, 0) => WallOrientation::Vertical,
        _ => unreachable!(),
    };

    let wall_xs = [x - 1 + (dx == 1) as i8, x - (dx == -1) as i8];
    let wall_ys = [y - 1 + (dy == 1) as i8, y - (dy == -1) as i8];
    for (wx, wy) in wall_xs.into_iter().zip(wall_ys.into_iter()) {
        if wx < 0 || wx >= WALL_GRID_WIDTH as i8 || wy < 0 || wy >= WALL_GRID_HEIGHT as i8 {
            // Out of bounds wall cannot exist / block movement.
            continue;
        }
        if game.board.walls.0[wx as usize][wy as usize] == Some(orientation) {
            return true;
        }
    }

    false
}

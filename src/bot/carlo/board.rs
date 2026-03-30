use std::fmt::Debug;

use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
    WallOrientation,
};

use super::path::{Dir, PathBlock};

pub struct Board {
    game: Game,
    path: [[(PathBlock, u8); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    wall_moves: [[[bool; 2]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
    wall_move_count: usize,
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.path {
            for square in row {
                let _ = write!(f, "{:3}{:?} ", square.1, square.0);
            }
            let _ = writeln!(f);
        }
        Ok(())
    }
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        let game = game.clone();

        let mut path = [[(PathBlock::Unreachable, 255); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        {
            let y = match game.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                path[y][x] = (PathBlock::Goal, 0);
                queue[queue_len] = (x as i8, y as i8);
                queue_len += 1;
            }
        }

        let mut board = Board {
            game,
            path,
            wall_moves: [[[true, true]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
            wall_move_count: 2 * WALL_GRID_WIDTH * WALL_GRID_HEIGHT,
        };

        for x in 0..WALL_GRID_WIDTH {
            for y in 0..WALL_GRID_HEIGHT {
                match board.game.board.walls.0[x][y] {
                    Some(orientation) => board.place_wall(x, y, orientation),
                    None => {}
                }
            }
        }

        board.bfs(queue, queue_len);

        board
    }
}

impl Board {
    // Performs BFS search from the elements of the queue. Elements must be increasing in distance to goal.
    fn bfs(
        &mut self,
        in_queue: [(i8, i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
        in_queue_len: usize,
    ) {
        let mut queue = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        let player_pos = self.game.board.player_position(self.game.player);

        let mut i = 0;
        let mut in_queue_i = 0;
        let mut iter_dist = self.path[in_queue[0].1 as usize][in_queue[0].0 as usize].1;
        'queue: while i < queue_len || in_queue_i < in_queue_len {
            // println!("pd: {}", iter_dist);
            // print!("q: ");
            // for j in i..queue_len {
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[queue[j].1 as usize][queue[j].0 as usize].1, queue[j].0, queue[j].1
            //     )
            // }
            // println!();
            // print!("i: ");
            // for j in in_queue_i..in_queue_len {
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[in_queue[j].1 as usize][in_queue[j].0 as usize].1,
            //         in_queue[j].0,
            //         in_queue[j].1,
            //     )
            // }
            // println!("\n");

            while in_queue_i < in_queue_len
                && (i == queue_len
                    || self.path[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize]
                        .1
                        == iter_dist)
            {
                iter_dist =
                    self.path[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize].1;

                queue[queue_len] = in_queue[in_queue_i];
                queue_len += 1;
                in_queue_i += 1;
            }

            // println!("d: {}", iter_dist);
            // print!("q: ");
            // for j in i..queue_len {
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[queue[j].1 as usize][queue[j].0 as usize].1, queue[j].0, queue[j].1
            //     )
            // }
            // println!();
            // print!("i: ");
            // for j in in_queue_i..in_queue_len {
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[in_queue[j].1 as usize][in_queue[j].0 as usize].1,
            //         in_queue[j].0,
            //         in_queue[j].1,
            //     )
            // }
            // println!("\n");

            let (x, y) = queue[i];
            for (dx, dy) in board_neighbors(&self.game, x, y) {
                let nx = x + dx;
                let ny = y + dy;

                if self.path[ny as usize][nx as usize].0 != PathBlock::Unreachable {
                    continue;
                }

                let dist = self.path[y as usize][x as usize].1 + 1;
                let dir = PathBlock::Dir(Dir::from((dx, dy)).reverse());
                self.path[ny as usize][nx as usize] = (dir, dist);

                if nx as usize == player_pos.x && y as usize == player_pos.y {
                    break 'queue;
                }

                iter_dist = dist;
                queue[queue_len] = (nx, ny);
                queue_len += 1;
            }

            i += 1;
        }
    }

    pub fn place_wall(&mut self, x: usize, y: usize, orientation: WallOrientation) {
        if !self.wall_moves[y][x][match orientation {
            WallOrientation::Horizontal => 0,
            WallOrientation::Vertical => 1,
        }] {
            panic!("wall already removed");
        }

        self.game.board.walls.0[x][y] = Some(orientation);

        self.wall_moves[y][x][0] = false;
        self.wall_moves[y][x][1] = false;
        self.wall_move_count -= 2;
        match orientation {
            WallOrientation::Horizontal => {
                if x > 0 {
                    self.wall_moves[y][x - 1][0] = false;
                    self.wall_move_count -= 1;
                }
                if x < WALL_GRID_WIDTH - 1 {
                    self.wall_moves[y][x + 1][0] = false;
                    self.wall_move_count -= 1;
                }
            }
            WallOrientation::Vertical => {
                if y > 0 {
                    self.wall_moves[y - 1][x][0] = false;
                    self.wall_move_count -= 1;
                }
                if y < WALL_GRID_HEIGHT - 1 {
                    self.wall_moves[y + 1][x][0] = false;
                    self.wall_move_count -= 1;
                }
            }
        }
    }

    pub fn recalculate_bfs(&mut self, x: usize, y: usize, orientation: WallOrientation) {
        self.place_wall(x, y, orientation);

        let mut invalid_q = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut invalid_q_len = 0;

        for ((x, y), towards_wall) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
            .into_iter()
            .zip(match orientation {
                WallOrientation::Horizontal => [Dir::Down, Dir::Down, Dir::Up, Dir::Up],
                WallOrientation::Vertical => [Dir::Right, Dir::Left, Dir::Right, Dir::Left],
            })
        {
            if self.path[y][x].0 == PathBlock::Dir(towards_wall) {
                invalid_q[invalid_q_len] = (x as i8, y as i8);
                invalid_q_len += 1;
            }
        }

        let mut i = 0;
        while i < invalid_q_len {
            let (x, y) = invalid_q[i];

            self.path[y as usize][x as usize] = (PathBlock::Unreachable, 255);

            for (dx, dy) in board_neighbors(&self.game, x, y) {
                let nx = x + dx;
                let ny = y + dy;

                let towards_invalid = Dir::from((dx, dy)).reverse();
                if self.path[ny as usize][nx as usize].0 == PathBlock::Dir(towards_invalid) {
                    invalid_q[invalid_q_len] = (nx, ny);
                    invalid_q_len += 1;
                }
            }

            i += 1;
        }

        let mut border_q = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut border_q_len = 0;
        let mut border_q_contains = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];

        // Seed BFS with best neighbor (if any) of all invalid squares.
        let mut i = 0;
        while i < invalid_q_len {
            let (x, y) = invalid_q[i];
            let mut best_neighbor: Option<((i8, i8), u8)> = None;

            for (dx, dy) in board_neighbors(&self.game, x, y) {
                let nx = x + dx;
                let ny = y + dy;

                if self.path[ny as usize][nx as usize].0 != PathBlock::Unreachable {
                    let dist = self.path[ny as usize][nx as usize].1;
                    if best_neighbor.map(|n| dist < n.1).unwrap_or(true) {
                        best_neighbor = Some(((nx, ny), dist));
                    }
                }
            }

            if let Some(((nx, ny), _)) = best_neighbor
                && !border_q_contains[ny as usize][nx as usize]
            {
                border_q_contains[ny as usize][nx as usize] = true;
                border_q[border_q_len] = (nx, ny);
                border_q_len += 1;
            }

            i += 1;
        }

        // println!("{:?}", self);

        let mut bfs_q = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut bfs_q_len = 0;
        // Sorted insert into seeded bfs queue.
        {
            let mut mi = 255;
            let mut ma = 0;
            for i in 0..border_q_len {
                let d = self.path[border_q[i].1 as usize][border_q[i].0 as usize].1;
                if d < mi {
                    mi = d;
                }
                if d > ma {
                    ma = d;
                }
            }

            for d in mi..=ma {
                for i in 0..border_q_len {
                    if d == self.path[border_q[i].1 as usize][border_q[i].0 as usize].1 {
                        bfs_q[bfs_q_len] = border_q[i];
                        bfs_q_len += 1;
                    }
                }
            }
        }

        self.bfs(bfs_q, bfs_q_len);
    }
}

/// Returns iterator over (dx, dy) for valid neighbors on the board.
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

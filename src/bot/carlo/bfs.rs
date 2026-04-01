use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
    WallOrientation,
};

use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl From<(i8, i8)> for Dir {
    fn from(dxdy: (i8, i8)) -> Self {
        match dxdy {
            (-1, _) => Dir::Left,
            (1, _) => Dir::Right,
            (_, -1) => Dir::Up,
            (_, 1) => Dir::Down,
            _ => unreachable!(),
        }
    }
}

impl Dir {
    pub fn reverse(&self) -> Dir {
        match self {
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathBlock {
    Unreachable,
    Goal,
    Dir(Dir),
}

impl Debug for PathBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "⊗"),
            Self::Goal => write!(f, "⊙"),
            Self::Dir(Dir::Left) => write!(f, "🠈"),
            Self::Dir(Dir::Right) => write!(f, "🠊"),
            Self::Dir(Dir::Up) => write!(f, "🠉"),
            Self::Dir(Dir::Down) => write!(f, "🠋"),
        }
    }
}

#[derive(Clone)]
pub struct Bfs {
    pub player: Player,
    pub path: [[(PathBlock, u8); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
}

impl From<&Game> for Bfs {
    fn from(game: &Game) -> Self {
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

        let mut bfs = Bfs {
            path,
            player: game.player,
        };
        bfs.bfs(game, queue, queue_len);

        bfs
    }
}

impl Bfs {
    // Performs BFS search from the elements of the queue. Elements must be increasing in distance to goal.
    fn bfs(
        &mut self,
        game: &Game,
        in_queue: [(i8, i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
        in_queue_len: usize,
    ) {
        let mut queue = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        let player_pos = game.board.player_position(self.player);

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
            for (dx, dy) in board_neighbors(game, x, y) {
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

    pub fn recalculate_bfs(
        &mut self,
        game: &Game,
        x: usize,
        y: usize,
        orientation: WallOrientation,
    ) {
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

            for (dx, dy) in board_neighbors(game, x, y) {
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

            for (dx, dy) in board_neighbors(game, x, y) {
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

        self.bfs(game, bfs_q, bfs_q_len);
    }
}

pub fn game_winner(game: &Game) -> Option<Player> {
    if game.board.player_positions[0].y == PIECE_GRID_HEIGHT - 1 {
        return Some(Player::White);
    }
    if game.board.player_positions[1].y == 0 {
        return Some(Player::Black);
    }
    None
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

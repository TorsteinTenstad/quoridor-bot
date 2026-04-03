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

    pub fn delta(&self) -> (isize, isize) {
        match self {
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
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

impl Debug for Bfs {
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

#[derive(Clone)]
pub struct Bfs {
    pub dir: (PathBlock, usize),
    pub player: Player,
    pub path: [[(PathBlock, u8); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    pub on_path: [[bool; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    pub queue: [(i8, i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
    pub queue_i: usize,
    pub queue_count: usize,
    invalid_q: [(i8, i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
    invalid_q_i: usize,
    invalid_q_len: usize,
}

impl From<&Game> for Bfs {
    fn from(game: &Game) -> Self {
        let mut path = [[(PathBlock::Unreachable, 255); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_count = 0;

        {
            let y = match game.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                path[y][x] = (PathBlock::Goal, 0);
                queue[queue_count] = (x as i8, y as i8);
                queue_count += 1;
            }
        }

        let mut bfs = Bfs {
            path,
            player: game.player,
            dir: (PathBlock::Unreachable, 255),
            on_path: [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
            queue,
            queue_i: 0,
            queue_count,
            invalid_q: [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
            invalid_q_i: 0,
            invalid_q_len: 0,
        };
        bfs.bfs(game);

        bfs
    }
}

impl Bfs {
    // Performs BFS search from the elements of the queue. Elements must be increasing in distance to goal.
    pub fn bfs(&mut self, game: &Game) {
        let mut queue = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queued = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        {
            let y = match self.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                queue[queue_len] = (x as i8, y as i8);
                queue_len += 1;
                queued[y][x] = true;
            }
        }

        let in_queue = self.queue;
        let mut in_queue_i = self.queue_i;
        let mut in_queue_count = self.queue_count;

        let player_pos = game.board.player_position(self.player);

        let mut i = 0;
        let mut iter_dist = 0;

        for i in 0..in_queue_count {
            let ii = (in_queue_i + i) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
            if self.path[in_queue[ii].1 as usize][in_queue[ii].0 as usize].0
                != PathBlock::Unreachable
            {
                iter_dist = self.path[in_queue[ii].1 as usize][in_queue[ii].0 as usize].1;
                break;
            }
        }
        'queue: while i < queue_len || in_queue_count > 0 {
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
            // for j in 0..in_queue_count {
            //     let jj = (in_queue_i + j) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[in_queue[jj].1 as usize][in_queue[jj].0 as usize].1,
            //         in_queue[jj].0,
            //         in_queue[jj].1,
            //     )
            // }
            // println!("\n");

            while in_queue_count > 0
                && (i == queue_len
                    || self.path[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize]
                        .1
                        == iter_dist)
            {
                if self.path[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize].0
                    != PathBlock::Unreachable
                    && !queued[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize]
                {
                    iter_dist = self.path[in_queue[in_queue_i].1 as usize]
                        [in_queue[in_queue_i].0 as usize]
                        .1;

                    queued[in_queue[in_queue_i].1 as usize][in_queue[in_queue_i].0 as usize] = true;
                    queue[queue_len] = in_queue[in_queue_i];
                    queue_len += 1;
                }
                in_queue_i = (in_queue_i + 1) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
                in_queue_count -= 1;
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
            if self.path[y as usize][x as usize].0 == PathBlock::Unreachable {
                i += 1;
                continue;
            }
            for (dx, dy) in board_neighbors_unchecked(x, y) {
                let nx = x + dx;
                let ny = y + dy;

                if self.path[ny as usize][nx as usize].0 != PathBlock::Unreachable {
                    continue;
                }

                if wall_blocks(game, x, y, dx, dy) {
                    continue;
                }

                let dist = self.path[y as usize][x as usize].1 + 1;
                let dir = PathBlock::Dir(Dir::from((dx, dy)).reverse());
                self.path[ny as usize][nx as usize] = (dir, dist);

                queue[queue_len] = (nx, ny);
                queue_len += 1;

                if nx as usize == player_pos.x && ny as usize == player_pos.y {
                    self.dir = (dir, dist as usize);
                    // println!("aa{:?}", self);
                    // println!("aaxy {} {}", player_pos.x, player_pos.y);
                    // println!("aaxy {} {}", nx, ny);
                    break 'queue;
                }

                iter_dist = dist;
                queued[ny as usize][nx as usize] = true;
            }

            i += 1;
        }

        if self.path[player_pos.y][player_pos.x].0 == PathBlock::Unreachable {
            return;
        }

        self.recalculate_path(game);

        self.queue = queue;
        self.queue_i = i;
        self.queue_count = queue_len - i;

        while in_queue_count > 0 {
            self.queue[self.queue_i] = in_queue[in_queue_i];
            self.queue_i = (self.queue_i + 1) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
            self.queue_count += 1;
            in_queue_i = (in_queue_i + 1) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
            in_queue_count -= 1;
        }
    }

    pub fn recalculate_path(&mut self, game: &Game) {
        let player_pos = game.board.player_position(self.player);

        if self.path[player_pos.y][player_pos.x].0 == PathBlock::Unreachable {
            return;
        }

        let mut xx = player_pos.x;
        let mut yy = player_pos.y;
        let mut d = self.path[yy][xx].1;
        self.on_path = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        self.on_path[yy][xx] = true;
        while d > 0 {
            // println!("xy {} {} {} {}", nx, ny, xx, yy);
            let (dx, dy) = match self.path[yy][xx].0 {
                PathBlock::Dir(d) => d.delta(),
                PathBlock::Goal => {
                    panic!("unexpected goal")
                }
                PathBlock::Unreachable => {
                    println!("{:?}", self);
                    println!("xy {} {}", player_pos.x, player_pos.y);
                    println!("xy {} {}", xx, yy);
                    panic!("unexpected unreachable")
                }
            };
            xx = (xx as isize + dx) as usize;
            yy = (yy as isize + dy) as usize;
            self.on_path[yy][xx] = true;
            d = self.path[yy][xx].1;
        }
    }

    pub fn recalculate_bfs(
        &mut self,
        game: &Game,
        x: usize,
        y: usize,
        orientation: WallOrientation,
    ) {
        let mut can_skip = true;
        let player_pos = game.board.player_position(self.player);

        for ((x, y), towards_wall) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
            .into_iter()
            .zip(match orientation {
                WallOrientation::Horizontal => [Dir::Down, Dir::Down, Dir::Up, Dir::Up],
                WallOrientation::Vertical => [Dir::Right, Dir::Left, Dir::Right, Dir::Left],
            })
        {
            if self.path[y][x].0 == PathBlock::Dir(towards_wall) {
                self.invalid_q[self.invalid_q_len] = (x as i8, y as i8);
                self.invalid_q_len += 1;
                self.path[y as usize][x as usize] = (PathBlock::Unreachable, 255);
                if x as usize == player_pos.x && y as usize == player_pos.y {
                    self.dir = (PathBlock::Unreachable, 255);
                }

                if self.on_path[y as usize][x as usize] {
                    can_skip = false;
                }
            }
        }

        if can_skip {
            return;
        }

        self.invalidate(game);

        if self.dir.0 != PathBlock::Unreachable {
            // Blocked path does not affect players shortest path.
            return;
        }
        self.re_bfs(game);
    }

    pub fn re_bfs(&mut self, game: &Game) {
        let mut border_q = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut border_q_len = 0;
        let mut border_q_contains = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];

        // Seed BFS with best neighbor (if any) of all invalid squares.
        let mut i = 0;
        while i < self.invalid_q_len {
            let (x, y) = self.invalid_q[i];
            let mut best_neighbor: Option<((i8, i8), u8)> = None;

            for (dx, dy) in board_neighbors_unchecked(x, y) {
                let nx = x + dx;
                let ny = y + dy;

                if self.path[ny as usize][nx as usize].0 != PathBlock::Unreachable {
                    continue;
                }

                if wall_blocks(game, x, y, dx, dy) {
                    continue;
                }

                let dist = self.path[ny as usize][nx as usize].1;
                if best_neighbor.map(|n| dist < n.1).unwrap_or(true) {
                    best_neighbor = Some(((nx, ny), dist));
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

        self.invalid_q_i = 0;
        self.invalid_q_len = 0;
        self.insert_queue(bfs_q, bfs_q_len);
        self.bfs(game);
    }

    pub fn invalidate(&mut self, game: &Game) {
        let player_pos = game.board.player_position(self.player);

        while self.invalid_q_i < self.invalid_q_len {
            let (x, y) = self.invalid_q[self.invalid_q_i];

            for (dx, dy) in board_neighbors_unchecked(x, y) {
                let nx = x + dx;
                let ny = y + dy;

                let towards_invalid = Dir::from((dx, dy)).reverse();
                if self.path[ny as usize][nx as usize].0 == PathBlock::Dir(towards_invalid) {
                    // print!(" {}{}", nx, ny);
                    // if self.invalid_q_len >= WALL_GRID_WIDTH * WALL_GRID_HEIGHT {
                    //     print!("ivq: ");
                    //     for i in 0..self.invalid_q_len {
                    //         print!("{}{} ", self.invalid_q[i].0, self.invalid_q[i].1);
                    //     }
                    // }

                    self.path[ny as usize][nx as usize] = (PathBlock::Unreachable, 255);
                    if nx as usize == player_pos.x && ny as usize == player_pos.y {
                        self.dir = (PathBlock::Unreachable, 255);
                    }

                    self.invalid_q[self.invalid_q_len] = (nx, ny);
                    self.invalid_q_len += 1;
                }
            }

            self.invalid_q_i += 1;
        }
    }

    fn insert_queue(
        &mut self,
        queue: [(i8, i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
        queue_len: usize,
    ) {
        // print!("exx: {} {}", self.queue_count, queue_len);
        // for ji in 0..self.queue_count {
        //     let j = (self.queue_i + ji) % (PIECE_GRID_HEIGHT * PIECE_GRID_WIDTH);
        //     print!(
        //         "{} ({},{}) ",
        //         self.path[self.queue[j].1 as usize][self.queue[j].0 as usize].1,
        //         self.queue[j].0,
        //         self.queue[j].1
        //     );
        // }
        // println!();
        // print!("in ");
        // for j in 0..queue_len {
        //     print!(
        //         "{} ({},{}) ",
        //         self.path[queue[j].1 as usize][queue[j].0 as usize].1, queue[j].0, queue[j].1
        //     )
        // }
        // println!("");

        let mut queued = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];

        let mut merged = [(0_i8, 0_i8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut merged_len = 0;
        let mut j = 0;

        while self.queue_count > 0 && j < queue_len {
            if self.path[self.queue[self.queue_i].1 as usize][self.queue[self.queue_i].0 as usize].1
                < self.path[queue[j].1 as usize][queue[j].0 as usize].1
            {
                if !queued[self.queue[self.queue_i].1 as usize][self.queue[self.queue_i].0 as usize]
                    && self.path[self.queue[self.queue_i].1 as usize]
                        [self.queue[self.queue_i].0 as usize]
                        .0
                        != PathBlock::Unreachable
                {
                    queued[self.queue[self.queue_i].1 as usize]
                        [self.queue[self.queue_i].0 as usize] = true;
                    merged[merged_len] = self.queue[self.queue_i];
                    merged_len += 1;
                }
                self.queue_i = (self.queue_i + 1) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
                self.queue_count -= 1;
            } else {
                if !queued[queue[j].1 as usize][queue[j].0 as usize]
                    && self.path[queue[j].1 as usize][queue[j].0 as usize].0
                        != PathBlock::Unreachable
                {
                    queued[queue[j].1 as usize][queue[j].0 as usize] = true;
                    merged[merged_len] = queue[j];
                    merged_len += 1;
                }
                j += 1;
            }
        }
        while self.queue_count > 0 {
            if !queued[self.queue[self.queue_i].1 as usize][self.queue[self.queue_i].0 as usize]
                && self.path[self.queue[self.queue_i].1 as usize]
                    [self.queue[self.queue_i].0 as usize]
                    .0
                    != PathBlock::Unreachable
            {
                queued[self.queue[self.queue_i].1 as usize][self.queue[self.queue_i].0 as usize] =
                    true;
                merged[merged_len] = self.queue[self.queue_i];
                merged_len += 1;
            }
            self.queue_i = (self.queue_i + 1) % (PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT);
            self.queue_count -= 1;
        }
        while j < queue_len {
            if !queued[queue[j].1 as usize][queue[j].0 as usize]
                && self.path[queue[j].1 as usize][queue[j].0 as usize].0 != PathBlock::Unreachable
            {
                queued[queue[j].1 as usize][queue[j].0 as usize] = true;
                merged[merged_len] = queue[j];
                merged_len += 1;
            }
            j += 1;
        }
        self.queue = merged;
        self.queue_i = 0;
        self.queue_count = merged_len;

        // print!("res: ");
        // let mut j = self.queue_i;
        // while j < self.queue_end {
        //     print!(
        //         "{} ({},{}) ",
        //         self.path[self.queue[j].1 as usize][self.queue[j].0 as usize].1,
        //         self.queue[j].0,
        //         self.queue[j].1
        //     );
        //     j = (j + 1) % (PIECE_GRID_HEIGHT * PIECE_GRID_WIDTH)
        // }
        // println!();
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
    board_neighbors_unchecked(x, y).filter(move |(dx, dy)| !wall_blocks(game, x, y, *dx, *dy))
}

fn board_neighbors_unchecked(x: i8, y: i8) -> impl Iterator<Item = (i8, i8)> {
    [(-1, 0), (0, -1), (1, 0), (0, 1)]
        .into_iter()
        .filter(move |(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;

            if nx < 0 || nx >= PIECE_GRID_WIDTH as i8 || ny < 0 || ny >= PIECE_GRID_HEIGHT as i8 {
                // Invalid out of bounds move.
                return false;
            }

            true
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

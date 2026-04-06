use super::{
    board::{Dir, board_neighbors_unchecked, wall_blocks},
    buffer::Buffer,
};
use crate::data_model::{Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, WallOrientation};
use std::fmt::Debug;

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
                write!(f, "{:3}{:?} ", square.1, square.0).unwrap();
            }
            writeln!(f).unwrap();
        }
        Ok(())
    }
}

pub struct PrintableBfs {
    game: Game,
    bfs: Bfs,
}

impl Debug for PrintableBfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..PIECE_GRID_HEIGHT {
            for x in 0..PIECE_GRID_WIDTH {
                write!(f, "{:3}{:?}", self.bfs.path[y][x].1, self.bfs.path[y][x].0).unwrap();
                let invalid_q = (0..self.bfs.invalid_q_len)
                    .any(|i| self.bfs.invalid_q[i] == (x as i8, y as i8));
                let mut filler = 0;
                if invalid_q {
                    write!(f, "i").unwrap();
                } else {
                    filler += 1;
                }
                let search_q = self
                    .bfs
                    .queue
                    .iter()
                    .any(|(a, b)| *a == (x as i8) && *b == (y as i8));
                if search_q {
                    write!(f, "s").unwrap();
                } else {
                    filler += 1;
                }
                if self.bfs.on_path[y][x] {
                    write!(f, "p").unwrap();
                } else {
                    filler += 1;
                }
                for _ in 0..filler {
                    write!(f, " ").unwrap();
                }
                if wall_blocks(&self.game, x as i8, y as i8, 1, 0) {
                    write!(f, "|").unwrap();
                } else {
                    write!(f, " ").unwrap();
                }
            }
            writeln!(f).unwrap();
            if y < PIECE_GRID_HEIGHT - 1 {
                for x in 0..PIECE_GRID_WIDTH {
                    if wall_blocks(&self.game, x as i8, y as i8, 0, 1) {
                        write!(f, "------- ").unwrap();
                    } else {
                        write!(f, "        ").unwrap();
                    }
                }
                writeln!(f).unwrap();
            }
        }
        Ok(())
    }
}

const SQUARES: usize = PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT;

#[derive(Clone)]
pub struct Bfs {
    pub dir: (PathBlock, usize),
    pub player: Player,
    pub path: [[(PathBlock, u8); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    pub on_path: [[bool; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    pub queue: Buffer<(i8, i8), SQUARES>,
    invalid_q: [(i8, i8); SQUARES],
    invalid_q_i: usize,
    invalid_q_len: usize,
}

impl From<&Game> for Bfs {
    fn from(game: &Game) -> Self {
        let mut path = [[(PathBlock::Unreachable, 255); PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue: Buffer<(i8, i8), 81> = Buffer::<(i8, i8), SQUARES>::default();

        {
            let y = match game.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                path[y][x] = (PathBlock::Goal, 0);
                queue.insert((x as i8, y as i8));
            }
        }

        let mut bfs = Bfs {
            path,
            player: game.player,
            dir: (PathBlock::Unreachable, 255),
            on_path: [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
            queue,
            invalid_q: [(0_i8, 0_i8); SQUARES],
            invalid_q_i: 0,
            invalid_q_len: 0,
        };
        bfs.bfs(game);

        bfs
    }
}

impl Bfs {
    pub fn printable(&self, game: &Game) -> PrintableBfs {
        PrintableBfs {
            game: game.clone(),
            bfs: self.clone(),
        }
    }

    // Performs BFS search from the elements of the queue. Elements must be increasing in distance to goal.
    pub fn bfs(&mut self, game: &Game) {
        let mut queue = Buffer::<(i8, i8), SQUARES>::default();
        let mut queued = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];

        {
            let y = match self.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                queue.insert((x as i8, y as i8));
                queued[y][x] = true;
            }
        }

        let player_pos = game.board.player_position(self.player);

        let mut iter_dist = 0;
        for (x, y) in self.queue.iter() {
            let p = self.path[*y as usize][*x as usize];
            if p.0 != PathBlock::Unreachable {
                iter_dist = p.1;
                break;
            }
        }
        'queue: while queue.non_empty() || self.queue.non_empty() {
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
            //     let jj = (in_queue_i + j) % (SQUARES);
            //     print!(
            //         "{} ({},{}) ",
            //         self.path[in_queue[jj].1 as usize][in_queue[jj].0 as usize].1,
            //         in_queue[jj].0,
            //         in_queue[jj].1,
            //     )
            // }
            // println!("\n");

            while self.queue.non_empty()
                && (queue.empty() || {
                    let (head_x, head_y) = {
                        let (x, y) = self.queue.peek_first();
                        (*x as usize, *y as usize)
                    };
                    self.path[head_y][head_x].1 == iter_dist
                })
            {
                let (xi, yi) = self.queue.pop_first();
                let (x, y) = (xi as usize, yi as usize);
                if self.path[y][x].0 != PathBlock::Unreachable && !queued[y][x] {
                    iter_dist = self.path[y][x].1;

                    queued[y][x] = true;
                    queue.insert((xi, yi));
                }
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

            let (x, y) = queue.pop_first();
            if self.path[y as usize][x as usize].0 == PathBlock::Unreachable {
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

                queue.insert((nx, ny));

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
        }

        if self.path[player_pos.y][player_pos.x].0 == PathBlock::Unreachable {
            return;
        }

        self.recalculate_path(game);

        while !self.queue.empty() {
            queue.insert(self.queue.pop_first());
        }
        self.queue = queue;
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

        self.seed_bfs(game);
        self.bfs(game);
    }

    pub fn seed_bfs(&mut self, game: &Game) {
        let mut border_q = [(0_i8, 0_i8); SQUARES];
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

                if self.path[ny as usize][nx as usize].0 == PathBlock::Unreachable {
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

        let mut bfs_q = [(0_i8, 0_i8); SQUARES];
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

    fn insert_queue(&mut self, queue: [(i8, i8); SQUARES], queue_len: usize) {
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
        let mut merged = Buffer::<(i8, i8), SQUARES>::default();

        let mut j = 0;
        while self.queue.non_empty() && j < queue_len {
            let (head_x, head_y) = {
                let (x, y) = self.queue.peek_first();
                (*x as usize, *y as usize)
            };

            if self.path[head_y][head_x].1 < self.path[queue[j].1 as usize][queue[j].0 as usize].1 {
                let (xi, yi) = self.queue.pop_first();

                let (x, y) = (xi as usize, yi as usize);
                if !queued[y][x] && self.path[y][x].0 != PathBlock::Unreachable {
                    queued[y][x] = true;
                    merged.insert((xi, yi));
                }
            } else {
                let (xi, yi) = queue[j];
                j += 1;

                let (x, y) = (xi as usize, yi as usize);
                if !queued[y][x] && self.path[y][x].0 != PathBlock::Unreachable {
                    queued[y][x] = true;
                    merged.insert((xi, yi));
                }
            }
        }
        while self.queue.non_empty() {
            let (xi, yi) = self.queue.pop_first();

            let (x, y) = (xi as usize, yi as usize);
            if !queued[y][x] && self.path[y][x].0 != PathBlock::Unreachable {
                queued[y][x] = true;
                merged.insert((xi, yi));
            }
        }
        while j < queue_len {
            let (xi, yi) = queue[j];
            j += 1;

            let (x, y) = (xi as usize, yi as usize);
            if !queued[y][x] && self.path[y][x].0 != PathBlock::Unreachable {
                queued[y][x] = true;
                merged.insert((xi, yi));
            }
        }

        self.queue = merged;

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

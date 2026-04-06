use super::{
    bfs::{Bfs, PathBlock},
    iter::ABIter,
};
use crate::{
    bot::carlo::iter::ABCIter,
    data_model::{
        Game, MovePiece, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, PlayerMove, WALL_GRID_HEIGHT,
        WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::execute_move_unchecked,
};

#[derive(Clone)]
pub struct Board {
    pub game: Game,
    pub bfs_white: Bfs,
    pub bfs_black: Bfs,
    wall_moves: [[[bool; 2]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
}

#[derive(Debug, Default, Clone)]
pub struct BoardStats {
    pub dist: [usize; 2],
    pub walls: [usize; 2],
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        // TODO: one redundant clone
        let mut game_white = game.clone();
        let mut game_black = game.clone();
        game_white.player = Player::White;
        game_black.player = Player::Black;

        let mut board = Board {
            game: game.clone(),
            bfs_white: Bfs::from(&game_white),
            bfs_black: Bfs::from(&game_black),
            wall_moves: [[[true, true]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
        };

        for x in 0..WALL_GRID_WIDTH {
            for y in 0..WALL_GRID_HEIGHT {
                match board.game.board.walls.0[x][y] {
                    Some(orientation) => board.place_wall(x, y, orientation),
                    None => {}
                }
            }
        }

        board
    }
}

impl Board {
    pub fn all_moves(&self) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        self.piece_moves().chain(self.wall_moves())
    }

    pub fn piece_moves(&self) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        self.piece_moves_inner(false)
    }

    pub fn play_move(&mut self, m: PlayerMove) {
        let player = self.game.player;
        self.game = execute_move_unchecked(&self.game, &m);
        match m {
            PlayerMove::MovePiece(_) => {
                let pos = self.game.board.player_position(player);
                if player == Player::White {
                    self.bfs_white.invalidate(&self.game);
                    self.bfs_white.dir = (
                        self.bfs_white.path[pos.y][pos.x].0,
                        self.bfs_white.path[pos.y][pos.x].1 as usize,
                    );
                    if self.bfs_white.dir.0 == PathBlock::Unreachable {
                        self.bfs_white.seed_bfs(&self.game);
                        self.bfs_white.bfs(&self.game);
                    } else {
                        self.bfs_white.recalculate_path(&self.game);
                    }
                }
                if player == Player::Black {
                    self.bfs_black.invalidate(&self.game);
                    self.bfs_black.dir = (
                        self.bfs_black.path[pos.y][pos.x].0,
                        self.bfs_black.path[pos.y][pos.x].1 as usize,
                    );
                    if self.bfs_black.dir.0 == PathBlock::Unreachable {
                        self.bfs_black.seed_bfs(&self.game);
                        self.bfs_black.bfs(&self.game);
                    } else {
                        self.bfs_black.recalculate_path(&self.game);
                    }
                }
            }
            PlayerMove::PlaceWall {
                orientation,
                position,
            } => {
                self.place_wall(position.x, position.y, orientation);

                self.bfs_white
                    .recalculate_bfs(&self.game, position.x, position.y, orientation);
                self.bfs_black
                    .recalculate_bfs(&self.game, position.x, position.y, orientation);
            }
        }
    }

    pub fn non_wait_moves(&self) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        // FIXME: Not correct due to jumping over opponent.
        let is_closest = if self.game.player == Player::White {
            self.bfs_white.dir.1 <= self.bfs_black.dir.1
        } else {
            self.bfs_black.dir.1 <= self.bfs_white.dir.1
        };
        let opponent_empty = self.game.walls_left[self.game.player.opponent().as_index()] == 0;

        let piece_moves = self.piece_moves_inner(true);
        if is_closest && opponent_empty {
            ABIter::A(piece_moves)
        } else {
            ABIter::B(piece_moves.chain(self.wall_moves()))
        }
    }

    fn piece_moves_inner(
        &self,
        shortest: bool,
    ) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        let pos: &crate::data_model::PiecePosition =
            &self.game.board.player_positions[self.game.player.as_index()];
        let other_pos = &self.game.board.player_positions[self.game.player.opponent().as_index()];
        let bfs: &Bfs = match self.game.player {
            Player::White => &self.bfs_white,
            Player::Black => &self.bfs_black,
        };

        let moves =
            board_neighbors(&self.game, pos.x as i8, pos.y as i8).flat_map(move |(dx, dy)| {
                let x = pos.x as i8 + dx;
                let y = pos.y as i8 + dy;
                let dir = Dir::from((dx, dy));

                let jump_opponent = x == other_pos.x as i8 && y == other_pos.y as i8;
                if !jump_opponent {
                    // 2nd direction param in tuple is unused when not jumping.
                    let anything = dir;
                    ABCIter::A(std::iter::once((dir, anything, (x, y))))
                } else {
                    if !outside_board(x + dx, y + dy) && !wall_blocks(&self.game, x, y, dx, dy) {
                        ABCIter::B(std::iter::once((dir, dir, (x + dx, y + dy))))
                    } else {
                        ABCIter::C(
                            match (dx, dy) {
                                (_, 0) => [(0, -1), (0, 1)],
                                (0, _) => [(-1, 0), (1, 0)],
                                _ => unreachable!(),
                            }
                            .into_iter()
                            .filter_map(move |(dx2, dy2)| {
                                let x2 = x + dx2;
                                let y2 = y + dy2;

                                if outside_board(x2, y2) || wall_blocks(&self.game, x, y, dx2, dy2)
                                {
                                    return None;
                                }

                                Some((dir, Dir::from((dx2, dy2)), (x2, y2)))
                            }),
                        )
                    }
                }
            });

        let moves = if shortest {
            ABIter::A(
                moves
                    .min_by(|(_, _, (x, y)), (_, _, (x2, y2))| {
                        bfs.path[*y as usize][*x as usize]
                            .1
                            .cmp(&bfs.path[*y2 as usize][*x2 as usize].1)
                    })
                    .into_iter(),
            )
        } else {
            ABIter::B(moves)
        };

        moves.map(|(dir1, dir2, (x, y))| {
            let m = PlayerMove::MovePiece(MovePiece {
                direction: dir1.into(),
                direction_on_collision: dir2.into(),
            });

            let dist = if bfs.path[y as usize][x as usize].0 != PathBlock::Unreachable {
                bfs.path[y as usize][x as usize].1 as usize
            } else {
                // BFS terminates when reaching player - must continue.
                let mut b = self.clone();
                b.play_move(m.clone());
                match self.game.player {
                    Player::White => b.bfs_white.dir.1 as usize,
                    Player::Black => b.bfs_black.dir.1 as usize,
                }
            };

            (
                m,
                Some(BoardStats {
                    dist: match self.game.player {
                        Player::White => [dist, self.bfs_black.dir.1],
                        Player::Black => [self.bfs_white.dir.1, dist],
                    },
                    walls: self.game.walls_left,
                }),
            )
        })
    }

    pub fn wall_moves(&self) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        if self.game.walls_left[self.game.player.as_index()] == 0 {
            return ABIter::A(std::iter::empty());
        };

        ABIter::B(
            self.wall_moves
                .iter()
                .enumerate()
                .flat_map(move |(y, col)| {
                    col.iter().enumerate().flat_map(move |(x, ors)| {
                        ors.iter().enumerate().filter_map(move |(orient, allowed)| {
                            if !allowed {
                                return None;
                            }
                            let orientation =
                                [WallOrientation::Horizontal, WallOrientation::Vertical][orient];

                            let (valid, stats) = self.valid_wall(x, y, orientation);
                            if !valid {
                                return None;
                            }

                            Some((
                                PlayerMove::PlaceWall {
                                    orientation,
                                    position: WallPosition { x, y },
                                },
                                stats,
                            ))
                        })
                    })
                }),
        )
    }

    pub fn valid_wall(
        &self,
        x: usize,
        y: usize,
        orientation: WallOrientation,
    ) -> (bool, Option<BoardStats>) {
        let mut board = self.clone();
        board.place_wall(x, y, orientation);
        board.game.walls_left[self.game.player.as_index()] -= 1;

        board
            .bfs_white
            .recalculate_bfs(&board.game, x, y, orientation);
        if board.bfs_white.dir.0 == PathBlock::Unreachable {
            return (false, None);
        }

        board
            .bfs_black
            .recalculate_bfs(&board.game, x, y, orientation);
        if board.bfs_black.dir.0 == PathBlock::Unreachable {
            return (false, None);
        }

        (
            true,
            Some(BoardStats {
                dist: [board.bfs_white.dir.1, board.bfs_black.dir.1],
                walls: board.game.walls_left,
            }),
        )
    }

    fn place_wall(&mut self, x: usize, y: usize, orientation: WallOrientation) {
        if !self.wall_moves[y][x][match orientation {
            WallOrientation::Horizontal => 0,
            WallOrientation::Vertical => 1,
        }] {
            panic!("wall already removed");
        }

        self.game.board.walls.0[x][y] = Some(orientation);

        self.wall_moves[y][x][0] = false;
        self.wall_moves[y][x][1] = false;
        match orientation {
            WallOrientation::Horizontal => {
                if x > 0 {
                    self.wall_moves[y][x - 1][0] = false;
                }
                if x < WALL_GRID_WIDTH - 1 {
                    self.wall_moves[y][x + 1][0] = false;
                }
            }
            WallOrientation::Vertical => {
                if y > 0 {
                    self.wall_moves[y - 1][x][1] = false;
                }
                if y < WALL_GRID_HEIGHT - 1 {
                    self.wall_moves[y + 1][x][1] = false;
                }
            }
        }
    }
}

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
            _ => {
                println!("{} {} is not a dir", dxdy.0, dxdy.1);
                panic!()
            }
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

impl Into<crate::data_model::Direction> for Dir {
    fn into(self) -> crate::data_model::Direction {
        match self {
            Dir::Left => crate::data_model::Direction::Left,
            Dir::Right => crate::data_model::Direction::Right,
            Dir::Up => crate::data_model::Direction::Up,
            Dir::Down => crate::data_model::Direction::Down,
        }
    }
}

/// Returns iterator over (dx, dy) for valid neighbors on the board.
pub fn board_neighbors(game: &Game, x: i8, y: i8) -> impl Iterator<Item = (i8, i8)> {
    board_neighbors_unchecked(x, y).filter(move |(dx, dy)| !wall_blocks(game, x, y, *dx, *dy))
}

pub fn outside_board(x: i8, y: i8) -> bool {
    x < 0 || x >= PIECE_GRID_WIDTH as i8 || y < 0 || y >= PIECE_GRID_HEIGHT as i8
}

pub fn board_neighbors_unchecked(x: i8, y: i8) -> impl Iterator<Item = (i8, i8)> {
    [(-1, 0), (0, -1), (1, 0), (0, 1)]
        .into_iter()
        .filter(move |(dx, dy)| {
            if outside_board(x + dx, y + dy) {
                // Invalid out of bounds move.
                return false;
            }

            true
        })
}

pub fn wall_blocks(game: &Game, x: i8, y: i8, dx: i8, dy: i8) -> bool {
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

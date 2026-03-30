use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, PlayerMove, WALL_GRID_HEIGHT,
    WALL_GRID_WIDTH, WallOrientation, WallPosition, Walls,
};

#[derive(Copy, Clone, PartialEq)]
pub enum Dir {
    None,
    PosX,
    PosY,
    NegX,
    NegY,
}

impl Dir {
    fn reverse(&self) -> Dir {
        match self {
            Dir::PosX => Dir::NegX,
            Dir::PosY => Dir::NegY,
            Dir::NegX => Dir::PosX,
            Dir::NegY => Dir::PosY,
            Dir::None => Dir::None,
        }
    }
    fn apply(&self, (x, y): (usize, usize)) -> (usize, usize) {
        match self {
            Dir::PosX => (x + 1, y),
            Dir::PosY => (x, y + 1),
            Dir::NegX => (x - 1, y),
            Dir::NegY => (x, y - 1),
            Dir::None => (x, y),
        }
    }
    fn can_apply(&self, (x, y): (usize, usize)) -> bool {
        match self {
            Dir::PosX => x < PIECE_GRID_WIDTH - 1,
            Dir::PosY => y < PIECE_GRID_HEIGHT - 1,
            Dir::NegX => x > 0,
            Dir::NegY => y > 0,
            Dir::None => false,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Tile {
    Invalid,
    Valid(Dir, u8),
}

struct Board {
    tiles: [[Tile; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
}

pub fn get_move(game: &Game) {
    let wall_moves = get_wall_moves(game);
    println!("count: {:?}", wall_moves.len());
}

fn get_board(game: &Game) -> Board {
    let mut board = Board {
        tiles: [[Tile::Invalid; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    };

    let mut queue = [(0, 0); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
    let mut queue_len = 0;

    let y_target = match game.player {
        Player::Black => 0,
        Player::White => PIECE_GRID_HEIGHT - 1,
    };
    for x in 0..PIECE_GRID_WIDTH {
        board.tiles[y_target][x] = Tile::Valid(Dir::None, 0);
        queue[queue_len] = (x, y_target);
        queue_len += 1;
    }

    bfs(&game.board.walls, &mut board, queue, queue_len);

    board
}

fn outside(x: usize, y: usize) -> bool {
    x <= 0 || x >= PIECE_GRID_WIDTH || y <= 0 || y >= PIECE_GRID_HEIGHT
}

fn bfs(
    walls: &Walls,
    board: &mut Board,
    mut queue: [(usize, usize); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT],
    mut queue_len: usize,
) {
    let mut i = 0;
    while i < queue_len {
        let xy = queue[i];
        let from = board.tiles[xy.1][xy.0];

        let distance = match from {
            Tile::Invalid => {
                unreachable!()
            }
            Tile::Valid(_, d) => d + 1,
        };

        for dir in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
            if !dir.can_apply(xy) {
                continue;
            }
            let (x, y) = dir.apply(xy);

            match board.tiles[y][x] {
                Tile::Valid(_, _) => {
                    continue;
                }
                _ => {}
            }

            let dx = x as i8 - xy.0 as i8;
            let dy = y as i8 - xy.1 as i8;
            if wall_blocks(walls, xy.0, xy.1, dx, dy) {
                continue;
            }

            board.tiles[y][x] = Tile::Valid(dir.reverse(), distance);
            queue[queue_len] = (x, y);
            queue_len += 1;
        }

        i += 1;
    }
}

fn wall_blocks(walls: &Walls, x: usize, y: usize, dx: i8, dy: i8) -> bool {
    let orientation = match (dx, dy) {
        (0, _) => WallOrientation::Horizontal,
        (_, 0) => WallOrientation::Vertical,
        _ => unreachable!(),
    };

    let wall_xs = [x as i8 - 1 + (dx == 1) as i8, x as i8 - (dx == -1) as i8];
    let wall_ys = [y as i8 - 1 + (dy == 1) as i8, y as i8 - (dy == -1) as i8];
    for (wx, wy) in wall_xs.into_iter().zip(wall_ys.into_iter()) {
        if wx < 0 || wx >= WALL_GRID_WIDTH as i8 || wy < 0 || wy >= WALL_GRID_HEIGHT as i8 {
            // Out of bounds wall cannot exist / block movement.
            continue;
        }
        if walls.0[wx as usize][wy as usize] == Some(orientation) {
            return true;
        }
    }

    false
}

fn get_wall_moves(game: &Game) -> Vec<PlayerMove> {
    let mut wall_moves: Vec<PlayerMove> = Vec::new();

    let mut game = game.clone();
    let board = get_board(&game);

    for y in 0..WALL_GRID_HEIGHT {
        for x in 0..WALL_GRID_WIDTH {
            for orientation in [WallOrientation::Horizontal, WallOrientation::Vertical] {
                let position = WallPosition { x, y };

                if wall_collide(&game.board.walls, orientation, &position) {
                    println!(
                        "Collision: {:?}",
                        PlayerMove::PlaceWall {
                            orientation,
                            position,
                        }
                    );
                    continue;
                }

                if wall_untouched(&game.board.walls, orientation, &position) {
                    wall_moves.push(PlayerMove::PlaceWall {
                        orientation,
                        position,
                    });
                    continue;
                }

                let mut board = Board {
                    tiles: board.tiles.clone(),
                };
                game.board.walls.0[position.x][position.y] = Some(orientation);

                let candidates = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
                    .into_iter()
                    .zip(match orientation {
                        WallOrientation::Horizontal => [Dir::PosY, Dir::PosY, Dir::NegY, Dir::NegY],
                        WallOrientation::Vertical => [Dir::PosX, Dir::NegX, Dir::PosX, Dir::NegX],
                    });

                for ((x, y), towards_wall) in candidates {
                    match board.tiles[y][x] {
                        Tile::Valid(dir, _) => {
                            if dir == towards_wall {
                                board_propagate_invalid(&mut board, x, y);
                            }
                        }
                        _ => {}
                    }
                }

                // TODO: simplify
                {
                    let mut queue = [(0, 0); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
                    let mut queue_len = 0;

                    let y_target = match game.player {
                        Player::Black => 0,
                        Player::White => PIECE_GRID_HEIGHT - 1,
                    };
                    for x in 0..PIECE_GRID_WIDTH {
                        board.tiles[y_target][x] = Tile::Valid(Dir::None, 0);
                        queue[queue_len] = (x, y_target);
                        queue_len += 1;
                    }

                    bfs(&game.board.walls, &mut board, queue, queue_len);
                }
                game.board.walls.0[position.x][position.y] = None;

                let mut skip = false;
                for pos in [
                    game.board.player_position(game.player),
                    game.board.player_position(game.player.opponent()),
                ] {
                    match board.tiles[pos.y][pos.x] {
                        Tile::Invalid => {
                            skip = true;
                        }
                        _ => {}
                    }
                }
                if skip {
                    println!(
                        "Breaks path: {:?}",
                        PlayerMove::PlaceWall {
                            orientation,
                            position,
                        }
                    );
                    continue;
                }

                wall_moves.push(PlayerMove::PlaceWall {
                    orientation,
                    position,
                });
            }
        }
    }

    wall_moves
}

fn board_propagate_invalid(board: &mut Board, x: usize, y: usize) {
    board.tiles[y][x] = Tile::Invalid;

    for dir_out in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
        if !dir_out.can_apply((x, y)) {
            continue;
        }
        let (x, y) = dir_out.apply((x, y));

        match board.tiles[y][x] {
            Tile::Valid(dir_in, _) => {
                if dir_in == dir_out.reverse() {
                    board_propagate_invalid(board, x, y);
                }
            }
            _ => {}
        }
    }
}

fn wall_collide(walls: &Walls, orientation: WallOrientation, position: &WallPosition) -> bool {
    let x = position.x;
    let y = position.y;

    if let Some(_) = walls.0[x][y] {
        return true;
    }

    match orientation {
        WallOrientation::Horizontal => {
            if x > 0
                && let Some(o) = walls.0[x - 1][y]
                && o == WallOrientation::Horizontal
            {
                return true;
            }
            if x < WALL_GRID_WIDTH - 1
                && let Some(o) = walls.0[x + 1][y]
                && o == WallOrientation::Horizontal
            {
                return true;
            }
        }
        WallOrientation::Vertical => {
            if y > 0
                && let Some(o) = walls.0[x][y - 1]
                && o == WallOrientation::Vertical
            {
                return true;
            }
            if y < WALL_GRID_HEIGHT - 1
                && let Some(o) = walls.0[x][y + 1]
                && o == WallOrientation::Vertical
            {
                return true;
            }
        }
    }

    return false;
}

fn wall_untouched(walls: &Walls, orientation: WallOrientation, position: &WallPosition) -> bool {
    let x = position.x;
    let y = position.y;
    let mut touches = 0;

    match orientation {
        WallOrientation::Horizontal => {
            if x == 0 {
                touches += 1
            } else if x == WALL_GRID_WIDTH - 1 {
                touches += 1
            }
            if wall_ends_at(walls, &WallPosition { x, y }) {
                touches += 1
            }
            if x > 0 && wall_ends_at(walls, &WallPosition { x: x - 1, y }) {
                touches += 1
            }
            if x < WALL_GRID_WIDTH - 1 && wall_ends_at(walls, &WallPosition { x: x + 1, y }) {
                touches += 1
            }
        }
        WallOrientation::Vertical => {
            if y == 0 {
                touches += 1
            } else if y == WALL_GRID_WIDTH - 1 {
                touches += 1
            }
            if wall_ends_at(walls, &WallPosition { x, y }) {
                touches += 1
            }
            if y > 0 && wall_ends_at(walls, &WallPosition { x, y: y - 1 }) {
                touches += 1
            }
            if y < WALL_GRID_HEIGHT - 1 && wall_ends_at(walls, &WallPosition { x, y: y + 1 }) {
                touches += 1
            }
        }
    }

    return touches < 2;
}

fn wall_ends_at(walls: &Walls, position: &WallPosition) -> bool {
    wall_collide(walls, WallOrientation::Horizontal, position)
        || wall_collide(walls, WallOrientation::Vertical, position)
}

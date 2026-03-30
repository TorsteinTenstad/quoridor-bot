use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, PlayerMove, WALL_GRID_HEIGHT,
    WALL_GRID_WIDTH, WallOrientation, WallPosition, Walls,
};
use std::collections::VecDeque;

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

#[derive(Clone)]
pub struct Board {
    pub tiles: [[Tile; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
}

pub fn get_move(game: &Game) {
    let wall_moves = _get_wall_moves(game);
    println!("count: {:?}", wall_moves.len());
}

pub fn get_board(game: &Game, player: Player) -> Board {
    let mut board = Board {
        tiles: [[Tile::Invalid; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    };

    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

    let y_target = match player {
        Player::Black => 0,
        Player::White => PIECE_GRID_HEIGHT - 1,
    };
    for x in 0..PIECE_GRID_WIDTH {
        board.tiles[y_target][x] = Tile::Valid(Dir::None, 0);
        queue.push_back((x, y_target));
    }

    bfs(&game.board.walls, &mut board, queue);

    board
}

fn bfs(walls: &Walls, board: &mut Board, mut queue: VecDeque<(usize, usize)>) {
    while let Some(xy) = queue.pop_front() {
        let from = board.tiles[xy.1][xy.0];

        let distance = match from {
            Tile::Invalid => {
                continue;
            }
            Tile::Valid(_, d) => d + 1,
        };

        for dir in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
            if !dir.can_apply(xy) {
                continue;
            }
            let (x, y) = dir.apply(xy);

            let dx = x as i8 - xy.0 as i8;
            let dy = y as i8 - xy.1 as i8;
            if wall_blocks(walls, xy.0, xy.1, dx, dy) {
                continue;
            }

            match board.tiles[y][x] {
                Tile::Valid(_, dis) => {
                    if dis <= distance {
                        continue;
                    }
                }
                _ => {}
            }

            board.tiles[y][x] = Tile::Valid(dir.reverse(), distance);
            queue.push_back((x, y));
        }
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
pub fn _get_wall_moves(game: &Game) -> Vec<(PlayerMove, Board, Board)> {
    let p1 = game.player;
    let p2 = game.player.opponent();

    let board_p1 = get_board(&game, p1);
    let board_p2 = get_board(&game, p2);

    get_wall_moves(game, &board_p1, &board_p2)
}

pub fn get_wall_moves(
    game: &Game,
    board_p1: &Board,
    board_p2: &Board,
) -> Vec<(PlayerMove, Board, Board)> {
    let mut wall_moves: Vec<(PlayerMove, Board, Board)> = Vec::new();

    let p1 = game.player;
    let p2 = game.player.opponent();
    if game.walls_left[p1.as_index()] == 0 {
        return wall_moves;
    }
    let mut game = game.clone();

    for y in 0..WALL_GRID_HEIGHT {
        for x in 0..WALL_GRID_WIDTH {
            for orientation in [WallOrientation::Horizontal, WallOrientation::Vertical] {
                let position = WallPosition { x, y };

                if wall_collide(&game.board.walls, orientation, &position) {
                    continue;
                }

                let pos1 = game.board.player_position(p1).clone();
                let pos2 = game.board.player_position(p2).clone();

                if wall_untouched(&game.board.walls, orientation, &position) {
                    wall_moves.push((
                        PlayerMove::PlaceWall {
                            orientation,
                            position,
                        },
                        board_p1.clone(),
                        board_p2.clone(),
                    ));
                    continue;
                }

                let board_p1 = board_after_wall(&mut game, &board_p1, x, y, orientation);
                match board_p1.tiles[pos1.y][pos1.x] {
                    Tile::Invalid => {
                        continue;
                    }
                    _ => {}
                }

                let board_p2 = board_after_wall(&mut game, &board_p2, x, y, orientation);
                match board_p2.tiles[pos2.y][pos2.x] {
                    Tile::Invalid => {
                        continue;
                    }
                    _ => {}
                }

                wall_moves.insert(
                    0,
                    (
                        PlayerMove::PlaceWall {
                            orientation,
                            position,
                        },
                        board_p1,
                        board_p2,
                    ),
                );
            }
        }
    }

    wall_moves
}

fn board_propagate_invalid(
    board: &mut Board,
    collect: &mut Vec<(usize, usize)>,
    x: usize,
    y: usize,
) {
    board.tiles[y][x] = Tile::Invalid;
    collect.push((x, y));

    for dir_out in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
        if !dir_out.can_apply((x, y)) {
            continue;
        }
        let (x, y) = dir_out.apply((x, y));

        match board.tiles[y][x] {
            Tile::Valid(dir_in, _) => {
                if dir_in == dir_out.reverse() {
                    board_propagate_invalid(board, collect, x, y);
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
                touches += 1;
            } else if x == WALL_GRID_WIDTH - 1 {
                touches += 1;
            }
            if wall_ends_at(walls, &WallPosition { x, y }) {
                touches += 1;
                if touches > 1 {
                    return false;
                }
            }
            if x > 0 && wall_ends_at(walls, &WallPosition { x: x - 1, y }) {
                touches += 1;
                if touches > 1 {
                    return false;
                }
            }
            if x < WALL_GRID_WIDTH - 1 && wall_ends_at(walls, &WallPosition { x: x + 1, y }) {
                touches += 1;
            }
        }
        WallOrientation::Vertical => {
            if y == 0 {
                touches += 1;
            } else if y == WALL_GRID_WIDTH - 1 {
                touches += 1;
            }
            if wall_ends_at(walls, &WallPosition { x, y }) {
                touches += 1;
                if touches > 1 {
                    return false;
                }
            }
            if y > 0 && wall_ends_at(walls, &WallPosition { x, y: y - 1 }) {
                touches += 1;
                if touches > 1 {
                    return false;
                }
            }
            if y < WALL_GRID_HEIGHT - 1 && wall_ends_at(walls, &WallPosition { x, y: y + 1 }) {
                touches += 1;
            }
        }
    }

    return touches < 2;
}

fn wall_ends_at(walls: &Walls, position: &WallPosition) -> bool {
    wall_collide(walls, WallOrientation::Horizontal, position)
        || wall_collide(walls, WallOrientation::Vertical, position)
}

fn board_after_wall(
    game: &mut Game,
    board: &Board,
    x: usize,
    y: usize,
    orientation: WallOrientation,
) -> Board {
    let mut board = board.clone();
    game.board.walls.0[x][y] = Some(orientation);

    let candidates = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
        .into_iter()
        .zip(match orientation {
            WallOrientation::Horizontal => [Dir::PosY, Dir::PosY, Dir::NegY, Dir::NegY],
            WallOrientation::Vertical => [Dir::PosX, Dir::NegX, Dir::PosX, Dir::NegX],
        });

    let mut invalids: Vec<(usize, usize)> = Vec::new();
    for ((x, y), towards_wall) in candidates {
        match board.tiles[y][x] {
            Tile::Valid(dir, _) => {
                if dir == towards_wall {
                    board_propagate_invalid(&mut board, &mut invalids, x, y);
                }
            }
            _ => {}
        }
    }

    {
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
        let mut seen = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];

        for invalid in invalids {
            for dir in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
                if !dir.can_apply(invalid) {
                    continue;
                }
                let (x, y) = dir.apply(invalid);

                if seen[y][x] {
                    continue;
                }
                seen[y][x] = true;

                let dx = x as i8 - invalid.0 as i8;
                let dy = y as i8 - invalid.1 as i8;
                if wall_blocks(&game.board.walls, invalid.0, invalid.1, dx, dy) {
                    continue;
                }

                match board.tiles[y][x] {
                    Tile::Valid(_, _) => {
                        queue.push_back((x, y));
                    }
                    _ => {}
                };
            }
        }

        bfs(&game.board.walls, &mut board, queue);
    }
    game.board.walls.0[x][y] = None;

    board
}

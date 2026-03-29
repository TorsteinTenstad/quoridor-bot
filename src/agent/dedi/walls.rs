use crate::{
    agent::dedi::bit_set::Bitset192,
    data_model::{
        Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, PiecePosition, Player, WALL_GRID_HEIGHT,
        WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct Wall {
    position: WallPosition,
    orientation: WallOrientation,
}

pub enum WallsBetween {
    Single(Wall),
    Double(Wall, Wall),
}

pub fn get_illegal_walls(game: &Game) -> Vec<Wall> {
    let position = game.board.player_position(game.player);

    let mut board_walls = Bitset192::new(0, 0, 0);
    for y in 0..WALL_GRID_HEIGHT {
        for x in 0..WALL_GRID_HEIGHT {
            let wall = game.board.walls.0[x][y];
            match wall {
                None => {}
                Some(orientation) => {
                    let index = wall_index(Wall {
                        position: { WallPosition { x, y } },
                        orientation,
                    });
                    board_walls.set_bit(index);
                }
            }
        }
    }

    let y_target = match game.player {
        Player::Black => 0,
        Player::White => PIECE_GRID_HEIGHT - 1,
    };

    // let a: Vec<Wall> = board_walls
    //     .iter_ones()
    //     .map(|index| wall_from_index(index))
    //     .collect();
    // println!("{:?}", a);

    let wall_bitsets = bfs(&board_walls, position, y_target);

    if wall_bitsets.len() == 0 {
        println!("No illegal walls :)");
        return Vec::new();
    }

    let mut a = !0u64;
    let mut b = !0u64;
    let mut c = !0u64;

    for bs in &wall_bitsets {
        a &= bs.a;
        b &= bs.b;
        c &= bs.c;
    }

    let walls: Vec<Wall> = Bitset192 { a, b, c }
        .iter_ones()
        .map(|index| wall_from_index(index))
        .collect();

    println!("Illegal!: {:?}", walls);

    walls
}

pub fn bfs(board_walls: &Bitset192, initial: &PiecePosition, y_target: usize) -> Vec<Bitset192> {
    let mut wall_paths: Vec<Bitset192> = Vec::new();
    let tile_path = Bitset192::new(0, 0, 0);
    let wall_path = Bitset192::new(0, 0, 0);

    let mut queue = VecDeque::new();
    queue.push_back((initial.clone(), tile_path, wall_path));

    while let Some((position, tile_path, wall_path)) = queue.pop_front() {
        let mut tile_path = tile_path;
        assert!(tile_path.get_bit(tile_index(&position)) == false);
        tile_path.set_bit(tile_index(&position));

        if position.y == y_target {
            for other in wall_paths.iter() {
                let join = wall_path & *other;
                if !join.any() {
                    println!("Disjoint. Done :)");
                    let a: Vec<Wall> = other
                        .iter_ones()
                        .map(|index| wall_from_index(index))
                        .collect();
                    let b: Vec<Wall> = wall_path
                        .iter_ones()
                        .map(|index| wall_from_index(index))
                        .collect();
                    println!("{:?}", a);
                    println!("{:?}", b);
                    return Vec::new();
                }
            }
            wall_paths.push(wall_path);
            continue;
        }

        let mut targets: Vec<PiecePosition> = Vec::new();
        if position.y > 0 {
            targets.push(PiecePosition {
                x: position.x,
                y: position.y - 1,
            });
        }
        if position.y < PIECE_GRID_HEIGHT - 1 {
            targets.push(PiecePosition {
                x: position.x,
                y: position.y + 1,
            });
        }
        if position.x > 0 {
            targets.push(PiecePosition {
                x: position.x - 1,
                y: position.y,
            });
        }
        if position.x < PIECE_GRID_WIDTH - 1 {
            targets.push(PiecePosition {
                x: position.x + 1,
                y: position.y,
            });
        }

        for target in targets {
            let target_index = tile_index(&target);
            if tile_path.get_bit(target_index) {
                continue;
            }

            let walls = walls_between(&position, &target);
            let mut wall_path = wall_path;
            match walls {
                WallsBetween::Single(wall) => {
                    wall_path.set_bit(wall_index(wall));
                }
                WallsBetween::Double(wall_a, wall_b) => {
                    wall_path.set_bit(wall_index(wall_a));
                    wall_path.set_bit(wall_index(wall_b));
                }
            }

            if (board_walls.clone() & wall_path).any() {
                // println!("Can't go trough walls");
                // println!("{:?}", board_walls);
                // println!("{:?}", wall_path);
                continue;
            }

            queue.push_back((target, tile_path, wall_path));
        }
    }

    wall_paths
}

fn tile_index(pos: &PiecePosition) -> usize {
    pos.y * PIECE_GRID_HEIGHT + pos.x
}

fn tile_from_index(index: usize) -> PiecePosition {
    PiecePosition {
        x: index % PIECE_GRID_HEIGHT,
        y: index / PIECE_GRID_HEIGHT,
    }
}

fn wall_index(wall: Wall) -> usize {
    match wall.orientation {
        WallOrientation::Horizontal => wall.position.y * WALL_GRID_HEIGHT + wall.position.x,
        WallOrientation::Vertical => {
            WALL_GRID_WIDTH * WALL_GRID_HEIGHT
                + (wall.position.y * WALL_GRID_HEIGHT + wall.position.x)
        }
    }
}

fn wall_from_index(index: usize) -> Wall {
    let horizontal_count = WALL_GRID_WIDTH * WALL_GRID_HEIGHT;

    if index < horizontal_count {
        // It's a horizontal wall
        let y = index / WALL_GRID_HEIGHT;
        let x = index % WALL_GRID_HEIGHT;

        Wall {
            orientation: WallOrientation::Horizontal,
            position: WallPosition { x, y },
        }
    } else {
        // It's a vertical wall
        let vertical_index = index - horizontal_count;
        let y = vertical_index / WALL_GRID_HEIGHT;
        let x = vertical_index % WALL_GRID_HEIGHT;

        Wall {
            orientation: WallOrientation::Vertical,
            position: WallPosition { x, y },
        }
    }
}

fn move_index(a: PiecePosition, b: PiecePosition) -> usize {
    assert!(tile_index(&a) != tile_index(&b));

    let mut a = a;
    let mut b = b;
    if tile_index(&a) > tile_index(&b) {
        (a, b) = (b, a);
    }

    match (b.x - a.x, b.y - a.y) {
        (1, 0) => tile_index(&a),
        (0, 1) => PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT + tile_index(&a),
        _ => panic!(),
    }
}

fn walls_between(a: &PiecePosition, b: &PiecePosition) -> WallsBetween {
    let mut a = a;
    let mut b = b;
    if tile_index(&a) > tile_index(&b) {
        (a, b) = (b, a);
    }

    let orientation = match (b.x - a.x, b.y - a.y) {
        (1, 0) => WallOrientation::Vertical,
        (0, 1) => WallOrientation::Horizontal,
        _ => panic!(),
    };

    match orientation {
        WallOrientation::Horizontal => {
            if a.x == 0 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                });
            }
            if a.x == PIECE_GRID_WIDTH - 1 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: a.x - 1, y: a.y },
                    orientation,
                });
            }
            return WallsBetween::Double(
                Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                },
                Wall {
                    position: WallPosition { x: a.x - 1, y: a.y },
                    orientation,
                },
            );
        }
        WallOrientation::Vertical => {
            if a.y == 0 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                });
            }
            if a.y == PIECE_GRID_WIDTH - 1 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: a.x, y: a.y - 1 },
                    orientation,
                });
            }
            return WallsBetween::Double(
                Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                },
                Wall {
                    position: WallPosition { x: a.x, y: a.y - 1 },
                    orientation,
                },
            );
        }
    }
}

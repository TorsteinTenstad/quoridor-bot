use crate::{
    agent::dedi::bit_set::Bitset192,
    data_model::{
        Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, PiecePosition, Player, WALL_GRID_HEIGHT,
        WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
};

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
            let wall = game.board.walls.0[y][x];
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

    let mut all_wall_paths: Vec<Bitset192> = Vec::new();
    let tile_path = Bitset192::new(0, 0, 0);
    let wall_path = Bitset192::new(0, 0, 0);
    let y_target = match game.player {
        Player::Black => 0,
        Player::White => PIECE_GRID_HEIGHT - 1,
    };

    let result = dfs(
        game,
        &board_walls,
        &mut all_wall_paths,
        tile_path,
        wall_path,
        position,
        y_target,
    );

    println!("{:?}", all_wall_paths.len());

    if match result {
        DfsResult::Unknown => false,
        DfsResult::EarlyReturn => true,
    } {
        return Vec::new();
    }

    let mut a = !0u64;
    let mut b = !0u64;
    let mut c = !0u64;

    for bs in &all_wall_paths {
        a &= bs.a;
        b &= bs.b;
        c &= bs.c;
    }

    Bitset192 { a, b, c }
        .iter_ones()
        .map(|index| wall_from_index(index))
        .collect()
}

pub fn dfs(
    game: &Game,
    board_walls: &Bitset192,
    all_wall_paths: &mut Vec<Bitset192>,
    tile_path: Bitset192,
    wall_path: Bitset192,
    position: &PiecePosition,
    y_target: usize,
) -> DfsResult {
    let mut tile_path = tile_path;
    assert!(tile_path.get_bit(tile_index(position)) == false);
    tile_path.set_bit(tile_index(position));

    println!("{:?}", tile_path.iter_ones().count());

    if position.y == y_target {
        for other in all_wall_paths.iter() {
            let join = wall_path & *other;
            if !join.any() {
                println!("DONE");
                return DfsResult::EarlyReturn;
            }
        }
        all_wall_paths.push(wall_path);
        return DfsResult::Unknown;
    }

    let mut targets: Vec<PiecePosition> = Vec::new();
    if position.y > y_target {
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
    } else {
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

        let walls = walls_between(position, &target);
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
            continue;
        }

        let result = dfs(
            game,
            board_walls,
            all_wall_paths,
            tile_path,
            wall_path,
            &target,
            y_target,
        );

        if match result {
            DfsResult::Unknown => false,
            DfsResult::EarlyReturn => true,
        } {
            return DfsResult::EarlyReturn;
        }
    }

    DfsResult::Unknown
}

pub enum DfsResult {
    Unknown,
    EarlyReturn,
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
        (1, 0) => WallOrientation::Horizontal,
        (0, 1) => WallOrientation::Vertical,
        _ => panic!(),
    };

    match orientation {
        WallOrientation::Horizontal => {
            if a.y == 0 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: a.x, y: 0 },
                    orientation,
                });
            }
            if a.y == PIECE_GRID_HEIGHT - 1 {
                return WallsBetween::Single(Wall {
                    position: WallPosition {
                        x: a.x,
                        y: WALL_GRID_HEIGHT - 1,
                    },
                    orientation,
                });
            }
            WallsBetween::Double(
                Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                },
                Wall {
                    position: WallPosition { x: a.x, y: a.y - 1 },
                    orientation,
                },
            )
        }
        WallOrientation::Vertical => {
            if a.x == 0 {
                return WallsBetween::Single(Wall {
                    position: WallPosition { x: 0, y: a.y },
                    orientation,
                });
            }
            if a.x == PIECE_GRID_WIDTH - 1 {
                return WallsBetween::Single(Wall {
                    position: WallPosition {
                        x: WALL_GRID_WIDTH - 1,
                        y: a.y,
                    },
                    orientation,
                });
            }
            WallsBetween::Double(
                Wall {
                    position: WallPosition { x: a.x, y: a.y },
                    orientation,
                },
                Wall {
                    position: WallPosition { x: a.x - 1, y: a.y },
                    orientation,
                },
            )
        }
    }
}

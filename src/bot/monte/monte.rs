use crate::{
    bot::dedi::walls::{Dir, get_board, get_wall_moves, wall_blocks, wall_collide},
    data_model::{
        Board, Game, MovePiece, PIECE_GRID_HEIGHT, PiecePosition, Player, PlayerMove,
        WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition, Walls,
    },
    game_logic::execute_move_unchecked_inplace,
};
use arrayvec::ArrayVec;
use rand::{Rng, seq::IndexedRandom};
use rand::{SeedableRng, rngs::SmallRng};
use rayon::prelude::*;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

pub fn monte(game: &Game, duration: Duration) -> PlayerMove {
    let deadline = Instant::now() + duration;

    let legal_moves: ArrayVec<PlayerMove, 136> = get_legal_moves(game).collect();
    let wall_moves: ArrayVec<PlayerMove, 128> = wall_moves_iter()
        .filter(|m| match m {
            PlayerMove::MovePiece(_) => false,
            PlayerMove::PlaceWall {
                orientation,
                position,
            } => !wall_collide(&game.board.walls, orientation.clone(), position),
        })
        .collect();

    let (win_count, iterations) = run_parallel(&game, &legal_moves, &wall_moves, deadline);

    let best_idx = win_count
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(i, _)| i)
        .expect("No legal moves");

    println!("{:?} / {:?}", win_count[best_idx], iterations);
    legal_moves[best_idx].clone()
}

fn run_parallel(
    game: &Game,
    legal_moves: &[PlayerMove],
    wall_moves: &[PlayerMove],
    deadline: Instant,
) -> (Vec<isize>, usize) {
    let mut win_count = vec![0isize; legal_moves.len()];
    let mut iterations = 0;

    while Instant::now() < deadline {
        // Run one batch in parallel
        let results: Vec<isize> = legal_moves
            .par_iter()
            .map(|m| {
                let mut rng = SmallRng::from_os_rng();
                simulate(&game, &mut rng, m.clone(), wall_moves)
            })
            .collect();

        // Merge results
        for (i, val) in results.into_iter().enumerate() {
            win_count[i] += val;
        }

        iterations += 1;
    }

    (win_count, iterations)
}

fn wall_moves_iter() -> impl Iterator<Item = PlayerMove> {
    [WallOrientation::Horizontal, WallOrientation::Vertical]
        .into_iter()
        .flat_map(|orientation| {
            (0..WALL_GRID_HEIGHT).flat_map(move |y| {
                (0..WALL_GRID_WIDTH).map(move |x| PlayerMove::PlaceWall {
                    orientation,
                    position: WallPosition { x, y },
                })
            })
        })
}

fn get_legal_moves(game: &Game) -> impl Iterator<Item = PlayerMove> {
    get_legal_piece_moves(game, game.player)
        .into_iter()
        .chain(get_legal_wall_moves(game))
}

pub fn get_legal_piece_moves(game: &Game, player: Player) -> ArrayVec<PlayerMove, 8> {
    let p1 = game.board.player_position(player);
    let p2 = game.board.player_position(player.opponent());
    get_legal_piece_moves_from_positions(&game.board.walls, p1, p2)
}

pub fn get_legal_piece_moves_from_positions(
    walls: &Walls,
    p1: &PiecePosition,
    p2: &PiecePosition,
) -> ArrayVec<PlayerMove, 8> {
    let p1 = (p1.x, p1.y);
    let p2 = (p2.x, p2.y);
    let mut moves: ArrayVec<PlayerMove, 8> = ArrayVec::new();

    let allow = |xy: (usize, usize), dir: Dir| {
        dir.can_apply(xy) && !wall_blocks(walls, xy.0 as isize, xy.1 as isize, dir)
    };

    for dir in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
        if !allow(p1, dir) {
            continue;
        }
        let (x, y) = dir.apply(p1);
        let direction = dir.to_direction();

        if x == p2.0 && y == p2.1 {
            if allow(p2, dir) {
                moves.push(PlayerMove::MovePiece(MovePiece {
                    direction: direction,
                    direction_on_collision: direction,
                }));
            } else {
                let (left, right) = dir.orthogonal();
                for _dir in [left, right] {
                    if allow(p2, _dir) {
                        moves.push(PlayerMove::MovePiece(MovePiece {
                            direction: direction,
                            direction_on_collision: _dir.to_direction(),
                        }));
                    }
                }
            }
        } else {
            moves.push(PlayerMove::MovePiece(MovePiece {
                direction: direction,
                direction_on_collision: direction,
            }));
        }
    }

    moves
}

pub fn get_legal_destinations(
    walls: &Walls,
    p1: (usize, usize),
    p2: (usize, usize),
) -> ArrayVec<(usize, usize), 8> {
    let mut moves: ArrayVec<(usize, usize), 8> = ArrayVec::new();

    let allow = |xy: (usize, usize), dir: Dir| {
        dir.can_apply(xy) && !wall_blocks(walls, xy.0 as isize, xy.1 as isize, dir)
    };

    for dir in [Dir::PosX, Dir::PosY, Dir::NegX, Dir::NegY] {
        if !allow(p1, dir) {
            continue;
        }
        let (x, y) = dir.apply(p1);

        if x == p2.0 && y == p2.1 {
            if allow(p2, dir) {
                moves.push(dir.apply(p2));
            } else {
                let (left, right) = dir.orthogonal();
                for _dir in [left, right] {
                    if allow(p2, _dir) {
                        moves.push(_dir.apply(p2));
                    }
                }
            }
        } else {
            moves.push(dir.apply(p1));
        }
    }

    moves
}

fn get_legal_wall_moves(game: &Game) -> impl Iterator<Item = PlayerMove> {
    let game = game.clone();
    let p1 = game.player;
    let p2 = game.player.opponent();
    let board_p1 = get_board(&game, p1);
    let board_p2 = get_board(&game, p2);

    get_wall_moves(&game, &board_p1, &board_p2)
        .into_iter()
        .map(|m| m.0)
}

const MAX_DEPTH: usize = 64;

fn simulate(
    game: &Game,
    rng: &mut SmallRng,
    move_initial: PlayerMove,
    wall_moves: &[PlayerMove],
) -> isize {
    let p1 = game.player;
    let p2 = game.player.opponent();
    let p1_target = target(p1);
    let p2_target = target(p2);
    let mut game = game.clone();
    let mut wall_moves: ArrayVec<_, 128> = wall_moves.iter().collect();

    execute_move_unchecked_inplace(&mut game, &move_initial);

    for _ in 1..MAX_DEPTH {
        let p1_pos = game.board.player_position(p1);
        if p1_pos.y == p1_target {
            return 1;
        }
        let p2_pos = game.board.player_position(p2);
        if p2_pos.y == p2_target {
            return -1;
        }
        if wall_moves.len() == 0 {
            break;
        }
        let no_walls_left = game.walls_left[game.player.as_index()] == 0;

        let m = if no_walls_left || rng.random_bool(0.8) {
            let piece_moves = get_legal_piece_moves(&game, game.player);
            if piece_moves.len() == 0 {
                return 0;
            }
            piece_moves.choose(rng).unwrap().clone()
        } else {
            let idx = rng.random_range(0..wall_moves.len());
            wall_moves.swap_remove(idx).clone()
        };

        execute_move_unchecked_inplace(&mut game, &m);
    }

    if wall_moves.len() > 0 {
        return 0;
    }

    let a = a_star_distance(&game.board, game.player);
    let b = a_star_distance(&game.board, game.player.opponent());

    if a <= b {
        if game.player == p1 { 1 } else { -1 }
    } else {
        if game.player == p1 { -1 } else { 1 }
    }
}

fn target(player: Player) -> usize {
    if player == Player::White {
        PIECE_GRID_HEIGHT - 1
    } else {
        0
    }
}

pub fn a_star_distance(board: &Board, player: Player) -> Option<usize> {
    let start = board.player_position(player).clone();
    let start = (start.x, start.y);
    let goal_h = heuristic(&start, player);

    let mut open_set = BinaryHeap::new();
    open_set.push((Reverse(goal_h), start.clone()));

    let mut g_score = HashMap::<(usize, usize), usize>::new();
    g_score.insert(start, 0);

    let opponent_position = board.player_position(player.opponent());
    let opponent_position = (opponent_position.x, opponent_position.y);

    while let Some((Reverse(_), current)) = open_set.pop() {
        let h = heuristic(&current, player);

        if h == 0 {
            return Some(g_score[&current]);
        }

        let current_g = g_score[&current];

        let _neighbors = get_legal_destinations(&board.walls, current.clone(), opponent_position);

        for neighbor in _neighbors {
            let tentative_g = current_g + 1;

            if tentative_g < *g_score.get(&neighbor).unwrap_or(&usize::MAX) {
                g_score.insert(neighbor.clone(), tentative_g);

                let f = tentative_g + heuristic(&neighbor, player);
                open_set.push((Reverse(f), neighbor));
            }
        }
    }

    None
}

pub fn heuristic(pos: &(usize, usize), player: Player) -> usize {
    match player {
        Player::White => PIECE_GRID_HEIGHT - 1 - pos.1,
        Player::Black => pos.1,
    }
}

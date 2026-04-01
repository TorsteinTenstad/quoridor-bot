use crate::{
    bot::dedi::walls::{Dir, get_board, get_wall_moves, wall_blocks, wall_collide},
    data_model::{
        Board, Game, MovePiece, PIECE_GRID_HEIGHT, PiecePosition, Player, PlayerMove,
        WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition, Walls,
    },
    game_logic::execute_move_unchecked_inplace,
};
use arrayvec::ArrayVec;
use rand::{Rng, seq::SliceRandom};
use rand::{SeedableRng, rngs::SmallRng};
use rayon::prelude::*;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, atomic::AtomicUsize};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

pub fn monte(game: &Game, duration: Duration) -> PlayerMove {
    let deadline = Instant::now() + duration;

    let legal_moves: ArrayVec<_, 136> = get_legal_moves(game).collect();
    let wall_moves: ArrayVec<_, 128> = wall_moves_iter()
        .filter(|m| match m {
            PlayerMove::MovePiece(_) => false,
            PlayerMove::PlaceWall {
                orientation,
                position,
            } => !wall_collide(&game.board.walls, orientation.clone(), position),
        })
        .collect();

    let (win_counts, iterations) = run_parallel(game, &legal_moves, &wall_moves, deadline);

    let win_rates: Vec<f32> = win_counts
        .iter()
        .zip(iterations.iter())
        .map(|(&wins, &iters)| {
            if iters == 0 {
                0.0
            } else {
                wins as f32 / iters as f32
            }
        })
        .collect();

    let mut indices: Vec<_> = (0..win_counts.len()).collect();
    indices.sort_by_key(|i| -win_counts[*i]);

    let top_three = indices.clone().into_iter().take(3);

    for idx in top_three {
        println!(
            "{:.1} % ({:?}): {:?}",
            win_rates[idx] * 100.0,
            iterations[idx],
            legal_moves[idx],
        );
    }

    let best_idx = *indices.first().unwrap();
    legal_moves[best_idx].clone()
}

const BATCH_ROUNDS: usize = 1_000;
const MIN_CANDIDATE_COUNT: usize = 2;
const RETAIN_RATIO: f32 = 0.8;

fn run_parallel(
    game: &Game,
    legal_moves: &[PlayerMove],
    wall_moves: &[PlayerMove],
    deadline: Instant,
) -> (Vec<isize>, Vec<usize>) {
    let count_all = legal_moves.len();
    let mut count_candidates = count_all;

    let win_counts: Vec<AtomicIsize> = (0..count_all).map(|_| AtomicIsize::new(0)).collect();
    let win_counts = Arc::new(win_counts);
    let iterations: Vec<AtomicUsize> = (0..count_all).map(|_| AtomicUsize::new(0)).collect();
    let iterations = Arc::new(iterations);

    let mut candidates: Vec<usize> = (0..count_all).collect();

    while Instant::now() < deadline && candidates.len() > 1 {
        let shared_candidates = Arc::new(candidates.clone());
        let win_counts_ref = win_counts.clone();
        let iterations_ref = iterations.clone();

        (0..rayon::current_num_threads())
            .into_par_iter()
            .for_each_init(
                || (SmallRng::from_os_rng(), vec![(0isize, 0usize); count_all]),
                |(rng, local), _| {
                    for _ in 0..BATCH_ROUNDS {
                        for &i in shared_candidates.iter() {
                            let r = simulate(game, rng, &legal_moves[i], wall_moves);
                            local[i].0 += r;
                            local[i].1 += 1;
                        }
                    }

                    for (i, (w, it)) in local.iter().enumerate() {
                        win_counts_ref[i].fetch_add(*w, Ordering::Relaxed);
                        iterations_ref[i].fetch_add(*it, Ordering::Relaxed);
                    }
                },
            );

        count_candidates =
            ((count_candidates as f32 * RETAIN_RATIO).floor() as usize).max(MIN_CANDIDATE_COUNT);

        candidates = (0..count_all).collect();
        candidates.sort_by(|&i, &j| {
            let a = win_counts[i].load(Ordering::Relaxed) as f32
                / iterations[i].load(Ordering::Relaxed) as f32;

            let b = win_counts[j].load(Ordering::Relaxed) as f32
                / iterations[j].load(Ordering::Relaxed) as f32;

            b.partial_cmp(&a).unwrap()
        });

        candidates.truncate(count_candidates);
    }

    (
        win_counts
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .collect(),
        iterations
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .collect(),
    )
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
    move_initial: &PlayerMove,
    wall_moves: &[PlayerMove],
) -> isize {
    let p1 = game.player;
    let p2 = game.player.opponent();
    let p1_target = target(p1);
    let p2_target = target(p2);
    let mut game = game.clone();

    let mut wall_move_indices: ArrayVec<usize, 128> = (0..wall_moves.len()).collect();
    wall_move_indices.shuffle(rng);
    let wall_move_count = wall_move_indices.len();
    let mut wall_move_idx_idx = 0;

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

        let walls_left = game.walls_left[game.player.as_index()];
        let walls_left_opponent = game.walls_left[game.player.opponent().as_index()];

        if wall_move_idx_idx >= wall_move_count || walls_left + walls_left_opponent == 0 {
            break;
        }

        let m = if walls_left == 0 || rng.random_bool(0.8) {
            let piece_moves = get_legal_piece_moves(&game, game.player);
            if piece_moves.len() == 0 {
                return 0;
            }

            let idx = rng.random_range(0..piece_moves.len());
            piece_moves[idx].clone()
        } else {
            let idx = wall_move_indices[wall_move_idx_idx];
            wall_move_idx_idx += 1;
            wall_moves[idx].clone()
        };

        execute_move_unchecked_inplace(&mut game, &m);
    }

    if wall_move_idx_idx < wall_move_count {
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

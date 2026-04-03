use crate::{
    bot::dedi::walls::{Dir, get_board, get_wall_moves, wall_blocks},
    data_model::{Game, MovePiece, PIECE_GRID_HEIGHT, PiecePosition, Player, PlayerMove, Walls},
    game_logic::execute_move_unchecked_inplace,
};
use arrayvec::ArrayVec;
use rand::{Rng, rngs::ThreadRng, seq::SliceRandom};
use rayon::prelude::*;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};
use std::{
    sync::{Arc, atomic::AtomicUsize},
    usize,
};

const BATCH_ROUNDS: usize = 1_000;
const MIN_CANDIDATE_COUNT: usize = 2;
const RETAIN_RATIO: f32 = 0.5;
const PIECE_PROBABILITY: f64 = 0.8;
const MAX_DEPTH: usize = 64;

const PIECE_MOVE_COUNT: usize = 6;

pub fn monte(game: &Game, duration: Duration) -> PlayerMove {
    let deadline = Instant::now() + duration;

    let mut rng = rand::rng();
    let mut legal_moves: ArrayVec<_, 136> = get_legal_moves(game).collect();
    legal_moves.shuffle(&mut rng);
    let mut wall_moves: ArrayVec<_, 128> = get_legal_wall_moves(game).collect();
    wall_moves.shuffle(&mut rng);

    let evaluations = run_parallel(game, &legal_moves, &wall_moves, deadline, BATCH_ROUNDS);

    let best_idx = evaluations.first().unwrap().move_index;
    for eval in evaluations.iter().take(10) {
        println!(
            "{:.1} % ({:?}/{:?}/{:?}): {:?}",
            if eval.iterations > 0 {
                100.0 * eval.win_count as f32 / eval.iterations as f32
            } else {
                0.0
            },
            eval.win_count,
            eval.iterations,
            eval.attempts,
            legal_moves[eval.move_index],
        );
    }
    println!("");

    legal_moves[best_idx].clone()
}

pub struct Evaluation {
    pub move_index: usize,
    pub win_count: isize,
    pub iterations: usize,
    pub attempts: usize,
}

pub fn run_parallel(
    game: &Game,
    legal_moves: &[PlayerMove],
    wall_moves: &[PlayerMove],
    deadline: Instant,
    batch_rounds: usize,
) -> Vec<Evaluation> {
    let count_all = legal_moves.len();
    let mut count_candidates = count_all;

    if count_all == 1 {
        return vec![Evaluation {
            move_index: 0,
            win_count: 0,
            iterations: 0,
            attempts: 0,
        }];
    }

    let win_counts: Vec<AtomicIsize> = (0..count_all).map(|_| AtomicIsize::new(0)).collect();
    let win_counts = Arc::new(win_counts);
    let iterations: Vec<AtomicUsize> = (0..count_all).map(|_| AtomicUsize::new(0)).collect();
    let iterations = Arc::new(iterations);
    let attempts: Vec<AtomicUsize> = (0..count_all).map(|_| AtomicUsize::new(0)).collect();
    let attempts = Arc::new(attempts);

    let mut candidates: Vec<usize> = (0..count_all).collect();

    loop {
        let shared_candidates = Arc::new(candidates.clone());
        let win_counts_ref = win_counts.clone();
        let iterations_ref = iterations.clone();
        let attempts_ref = attempts.clone();

        (0..rayon::current_num_threads())
            .into_par_iter()
            .for_each_init(
                || (rand::rng(), vec![(0isize, 0usize, 0usize); count_all]),
                |(rng, local), _| {
                    for _ in 0..batch_rounds {
                        for &i in shared_candidates.iter() {
                            if let Some(r) = simulate(game, rng, &legal_moves[i], wall_moves) {
                                local[i].0 += r;
                                local[i].1 += 1;
                            }
                            local[i].2 += 1;
                        }
                    }

                    for (i, (wins, iter, attem)) in local.iter().enumerate() {
                        win_counts_ref[i].fetch_add(*wins, Ordering::Relaxed);
                        iterations_ref[i].fetch_add(*iter, Ordering::Relaxed);
                        attempts_ref[i].fetch_add(*attem, Ordering::Relaxed);
                    }
                },
            );

        count_candidates =
            ((count_candidates as f32 * RETAIN_RATIO).floor() as usize).max(MIN_CANDIDATE_COUNT);

        candidates = (0..count_all).collect();
        candidates.sort_by(|&i, &j| {
            let a_iter = iterations[i].load(Ordering::Relaxed);
            let b_iter = iterations[j].load(Ordering::Relaxed);
            let a = if a_iter == 0 {
                0.0
            } else {
                win_counts[i].load(Ordering::Relaxed) as f32 / a_iter as f32
            };
            let b = if b_iter == 0 {
                0.0
            } else {
                win_counts[j].load(Ordering::Relaxed) as f32 / b_iter as f32
            };
            if a == b {
                let a_attem = attempts[i].load(Ordering::Relaxed);
                let b_attem = attempts[j].load(Ordering::Relaxed);
                let a = if a_attem == 0 {
                    0.0
                } else {
                    a as f32 / a_attem as f32
                };
                let b = if b_attem == 0 {
                    0.0
                } else {
                    b as f32 / b_attem as f32
                };
                b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        if Instant::now() >= deadline {
            return candidates
                .iter()
                .map(|i| Evaluation {
                    move_index: *i,
                    win_count: win_counts[*i].load(Ordering::Relaxed),
                    iterations: iterations[*i].load(Ordering::Relaxed),
                    attempts: attempts[*i].load(Ordering::Relaxed),
                })
                .collect();
        }

        candidates.truncate(count_candidates);
    }
}

fn get_legal_moves(game: &Game) -> impl Iterator<Item = PlayerMove> {
    get_legal_piece_moves(game, game.player)
        .into_iter()
        .map(PlayerMove::MovePiece)
        .chain(get_legal_wall_moves(game))
}

pub fn get_legal_piece_moves(game: &Game, player: Player) -> ArrayVec<MovePiece, PIECE_MOVE_COUNT> {
    let p1 = game.board.player_position(player);
    let p2 = game.board.player_position(player.opponent());
    get_legal_piece_moves_from_positions(&game.board.walls, p1, p2)
}

pub fn get_legal_piece_moves_from_positions(
    walls: &Walls,
    p1: &PiecePosition,
    p2: &PiecePosition,
) -> ArrayVec<MovePiece, PIECE_MOVE_COUNT> {
    let p1 = (p1.x, p1.y);
    let p2 = (p2.x, p2.y);
    let mut moves: ArrayVec<MovePiece, PIECE_MOVE_COUNT> = ArrayVec::new();

    let allow = |xy: (usize, usize), dir: Dir| {
        dir.can_apply(xy) && !wall_blocks(walls, xy.0 as isize, xy.1 as isize, dir)
    };

    for dir in [Dir::PosY, Dir::NegY, Dir::PosX, Dir::NegX] {
        if !allow(p1, dir) {
            continue;
        }
        let (x, y) = dir.apply(p1);
        let direction = dir.to_direction();

        if x == p2.0 && y == p2.1 {
            if allow(p2, dir) {
                moves.push(MovePiece {
                    direction: direction,
                    direction_on_collision: direction,
                });
            } else {
                let (left, right) = dir.orthogonal();
                for _dir in [left, right] {
                    if allow(p2, _dir) {
                        moves.push(MovePiece {
                            direction: direction,
                            direction_on_collision: _dir.to_direction(),
                        });
                    }
                }
            }
        } else {
            moves.push(MovePiece {
                direction: direction,
                direction_on_collision: direction,
            });
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

    for dir in [Dir::PosY, Dir::NegY, Dir::PosX, Dir::NegX] {
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

fn simulate(
    game: &Game,
    rng: &mut ThreadRng,
    move_initial: &PlayerMove,
    wall_moves: &[PlayerMove],
) -> Option<isize> {
    let p_a = game.player;
    let p_b = game.player.opponent();
    let target_a = target(p_a);
    let target_b = target(p_b);

    let mut game = game.clone();

    let mut wall_move_indices: ArrayVec<usize, 128> = (0..wall_moves.len()).collect();
    wall_move_indices.shuffle(rng);
    let wall_move_count = wall_move_indices.len();
    let mut wall_move_idx_idx = 0;

    execute_move_unchecked_inplace(&mut game, &move_initial);

    for _ in 0..MAX_DEPTH {
        let pos_a = game.board.player_position(p_a).clone();
        if pos_a.y == target_a {
            return Some(1);
        }
        let pos_b = game.board.player_position(p_b).clone();
        if pos_b.y == target_b {
            return Some(-1);
        }

        let p_i = game.player;
        let walls_i = game.walls_left[p_i.as_index()];

        let m = if walls_i == 0
            || wall_move_idx_idx >= wall_move_count
            || rng.random_bool(PIECE_PROBABILITY)
        {
            let piece_moves = get_legal_piece_moves(&game, game.player);
            if piece_moves.len() == 0 {
                return None;
            }

            let idx = rng.random_range(0..piece_moves.len());
            PlayerMove::MovePiece(piece_moves[idx].clone())
        } else {
            let idx = wall_move_indices[wall_move_idx_idx];
            wall_move_idx_idx += 1;
            wall_moves[idx].clone()
        };

        execute_move_unchecked_inplace(&mut game, &m);
    }

    return None;
}

fn target(player: Player) -> usize {
    if player == Player::White {
        PIECE_GRID_HEIGHT - 1
    } else {
        0
    }
}

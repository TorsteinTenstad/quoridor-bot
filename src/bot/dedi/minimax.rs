use crate::{
    bot::{
        dedi::walls::{Board, Tile, get_board, get_wall_moves},
        monte::monte::get_legal_piece_moves,
    },
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, new_position_after_move_piece_unchecked,
    },
};
use arrayvec::ArrayVec;
use std::hash::{Hash, Hasher};
use std::{
    collections::HashMap,
    hash::DefaultHasher,
    time::{Duration, Instant},
};

pub const INF: isize = isize::MAX - 1;

#[derive(Clone, PartialEq)]
enum CacheFlag {
    Exact,
    Lower, // beta cutoff — score is a lower bound
    Upper, // failed low — score is an upper bound
}

#[derive(Clone)]
pub struct CacheLine {
    depth: usize,
    play: PlayerMove,
    h: isize,
    flag: CacheFlag,
}

#[derive(Default)]
pub struct Cache {
    table: HashMap<u64, CacheLine>,
}

pub fn minimax_iterative(game: &Game, duration: Duration, cache: &mut Cache) -> Option<PlayerMove> {
    let deadline = Some(Instant::now() + duration);
    let mut depth = 2;
    let mut best_move: Option<PlayerMove> = None;

    loop {
        match minimax(game, depth, deadline, cache) {
            Some((_move, h)) => {
                println!("Depth {:?}: found {:?} with h={:?}", depth, _move, h);
                if _move == None {
                    break;
                }
                best_move = _move;
                depth += 1;
                if h >= INF || h <= -INF {
                    break;
                }
                if depth > 12 {
                    break;
                }
            }
            None => break,
        }
    }

    best_move
}

pub fn minimax(
    game: &Game,
    depth: usize,
    deadline: Option<Instant>,
    cache: &mut Cache,
) -> Option<(Option<PlayerMove>, isize)> {
    let board_p1 = get_board(game, game.player);
    let board_p2 = get_board(game, game.player.opponent());
    _minimax(game, depth, -INF, INF, deadline, board_p1, board_p2, cache)
}

fn _minimax(
    game: &Game,
    depth: usize,
    alpha: isize,
    beta: isize,
    deadline: Option<Instant>,
    board_p1: Board,
    board_p2: Board,
    cache: &mut Cache,
) -> Option<(Option<PlayerMove>, isize)> {
    if deadline.is_some_and(|d| Instant::now() > d) {
        return None;
    }

    let p1 = game.player;
    let p2 = game.player.opponent();
    let pos_p1 = game.board.player_position(p1);
    let pos_p2 = game.board.player_position(p2);

    if pos_p1.y == target(p1) {
        return Some((None, INF));
    }
    if pos_p2.y == target(p2) {
        return Some((None, -INF));
    }

    let mut alpha = alpha;

    let hash = hash_to_u64(game);
    if let Some(line) = cache.table.get(&hash).cloned() {
        if line.depth >= depth {
            match line.flag {
                CacheFlag::Exact => return Some((Some(line.play), line.h)),
                CacheFlag::Lower => alpha = alpha.max(line.h),
                CacheFlag::Upper => {
                    let beta = beta.min(line.h);
                    if alpha >= beta {
                        return Some((Some(line.play), line.h));
                    }
                }
            }
            if alpha >= beta {
                return Some((Some(line.play), line.h));
            }
        }
    }

    if depth == 0 {
        let h = heuristic(game, &board_p1, &board_p2);
        return Some((None, h));
    }

    let mut moves: ArrayVec<(PlayerMove, Board, Board), 136> =
        get_legal_piece_moves(game, game.player)
            .iter()
            .map(|move_piece| {
                (
                    PlayerMove::MovePiece(move_piece.clone()),
                    board_p1.clone(),
                    board_p2.clone(),
                )
            })
            .chain(get_wall_moves(game, &board_p1, &board_p2))
            .collect();

    if moves.is_empty() {
        unreachable!("no valid moves");
    }

    let cached_best = cache.table.get(&hash).and_then(|l| Some(l.play.clone()));

    moves.sort_by_key(|(mv, b1, b2)| {
        if cached_best.as_ref() == Some(mv) {
            return isize::MIN;
        }
        let pos_p1_next = match mv {
            PlayerMove::MovePiece(mp) => {
                &new_position_after_move_piece_unchecked(pos_p1, mp, pos_p2)
            }
            _ => pos_p1,
        };
        let a = match b1.tiles[pos_p1_next.y][pos_p1_next.x] {
            Tile::Valid(_, dis) => dis as isize,
            _ => unreachable!(),
        };
        let b = match b2.tiles[pos_p2.y][pos_p2.x] {
            Tile::Valid(_, dis) => dis as isize,
            _ => unreachable!(),
        };
        a - b
    });

    let original_alpha = alpha;
    let mut best_score = -INF;
    let mut best_move = moves[0].0.clone();

    for (_move, b1, b2) in moves {
        let game_next = execute_move_unchecked(game, &_move);
        match _minimax(
            &game_next,
            depth - 1,
            -beta,
            -alpha,
            deadline,
            b2,
            b1,
            cache,
        ) {
            Some((_, h_child)) => {
                let h = -h_child;
                if h > best_score {
                    best_score = h;
                    best_move = _move;
                }
                alpha = alpha.max(best_score);
                if alpha >= beta {
                    break; // beta cutoff
                }
            }
            None => return None, // deadline exceeded
        }
    }

    let flag = if best_score <= original_alpha {
        CacheFlag::Upper // never raised alpha — upper bound
    } else if best_score >= beta {
        CacheFlag::Lower // caused cutoff — lower bound
    } else {
        CacheFlag::Exact // within window
    };

    cache.table.insert(
        hash,
        CacheLine {
            depth,
            play: best_move.clone(),
            h: best_score,
            flag,
        },
    );

    Some((Some(best_move), best_score))
}

fn target(player: Player) -> usize {
    if player == Player::White {
        PIECE_GRID_HEIGHT - 1
    } else {
        0
    }
}

fn hash_to_u64<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn heuristic(game: &Game, b1: &Board, b2: &Board) -> isize {
    _heuristic(game, game.player, b1) - _heuristic(game, game.player.opponent(), b2)
}

fn _heuristic(game: &Game, player: Player, board: &Board) -> isize {
    let pos = game.board.player_position(player);
    let dis = match board.tiles[pos.y][pos.x] {
        Tile::Valid(_, dis) => dis,
        Tile::Invalid => unreachable!(),
    };
    if dis == 0 {
        return INF;
    }

    let mut h: isize = 0;

    h -= dis as isize * 10;
    h += game.walls_left[player.as_index()] as isize * 15;

    h
}

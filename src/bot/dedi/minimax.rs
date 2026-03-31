use crate::{
    bot::dedi::walls::{Board, Tile, get_board, get_wall_moves},
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_piece_legal_with_players_at_positions,
    },
};
use std::hash::{Hash, Hasher};
use std::{
    collections::HashMap,
    hash::DefaultHasher,
    time::{Duration, Instant},
};

pub const INF: isize = isize::MAX - 1;

pub fn minimax_iterative(game: &Game, duration: Duration, cache: &mut Cache) -> Option<PlayerMove> {
    let deadline = Some(Instant::now() + duration);
    let mut depth = 2;
    let mut best_move: Option<PlayerMove> = None;
    loop {
        if let Some((_move, h)) = minimax(game, depth, deadline, cache) {
            println!("Depth {:?}: found {:?} with h={:?}", depth, _move, h);
            best_move = _move;
            depth += 1;
            if h == INF || h == -INF {
                break;
            }
        } else {
            break;
        }
    }

    best_move
}

#[derive(Default)]
pub struct Cache {
    pub table: HashMap<u64, CacheLine>,
}

#[derive(Clone)]
pub struct CacheLine {
    pub depth: usize,
    pub play: Option<PlayerMove>,
    pub h: isize,
}

pub fn hash_to_u64<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn minimax(
    game: &Game,
    depth: usize,
    deadline: Option<Instant>,
    cache: &mut Cache,
) -> Option<(Option<PlayerMove>, isize)> {
    let board_p1 = get_board(game, game.player);
    let board_p2 = get_board(game, game.player.opponent());

    _minimax(
        game, depth, depth, -INF, INF, deadline, board_p1, board_p2, cache,
    )
}

fn target(player: Player) -> usize {
    if player == Player::White {
        PIECE_GRID_HEIGHT - 1
    } else {
        0
    }
}

fn _minimax(
    game: &Game,
    depth: usize,
    depth_initial: usize,
    alpha: isize,
    beta: isize,
    deadline: Option<Instant>,
    board_p1: Board,
    board_p2: Board,
    cache: &mut Cache,
) -> Option<(Option<PlayerMove>, isize)> {
    if deadline.is_some_and(|deadline| Instant::now() > deadline) {
        return None;
    }

    let p1 = game.player;
    let p2 = game.player.opponent();
    let pos_p1 = game.board.player_position(game.player);
    let pos_p2 = game.board.player_position(game.player.opponent());

    if depth <= 0 {
        let h = heuristic(game, &board_p1, &board_p2);
        return Some((None, h));
    }

    let hash = hash_to_u64(game);
    if cache.table.contains_key(&hash) {
        let line = cache.table[&hash].clone();
        if line.depth >= depth {
            return Some((line.play, line.h));
        }
    }

    if pos_p1.y == target(p1) {
        return Some((None, INF));
    }
    if pos_p2.y == target(p2) {
        return Some((None, -INF));
    }

    let mut moves: Vec<(PlayerMove, Board, Board)> = Vec::new();

    for move_piece in all_move_piece_moves(pos_p1, pos_p2) {
        let legal = is_move_piece_legal_with_players_at_positions(
            &game.board.walls,
            pos_p1,
            pos_p2,
            &move_piece,
        );

        if legal {
            moves.push((
                PlayerMove::MovePiece(move_piece),
                board_p1.clone(),
                board_p2.clone(),
            ));
        }
    }

    let mut alpha = alpha;
    let mut h_best = -INF;
    let mut move_best: Option<PlayerMove> = None;

    let skip = depth_initial % 2 == 1 && depth <= 1;
    if !skip {
        for (move_wall, boards_or_cached) in
            get_wall_moves(game, &board_p1, &board_p2, depth, cache)
        {
            match boards_or_cached {
                super::walls::BoardsOrCached::Boards(b1, b2) => {
                    moves.push((move_wall, b1, b2));
                }
                super::walls::BoardsOrCached::Cached(cache_line) => {
                    if cache_line.depth >= depth {
                        let h_inv = -cache_line.h;

                        if h_inv > h_best || move_best == None {
                            h_best = h_inv;
                            move_best = Some(move_wall);
                        }
                        alpha = alpha.max(h_best);
                    }
                }
            }
        }
    }

    if moves.len() == 0 {
        return Some((None, -INF));
    }

    for (_move, b1, b2) in moves {
        let game_next = execute_move_unchecked(game, &_move);

        if let Some((_, h_next)) = _minimax(
            &game_next,
            depth - 1,
            depth_initial,
            -beta,
            -alpha,
            deadline,
            b2,
            b1,
            cache,
        ) {
            let h_inv = -h_next;

            if h_inv > h_best || move_best == None {
                h_best = h_inv;
                move_best = Some(_move);
            }
            alpha = alpha.max(h_best);
            if alpha >= beta {
                break;
            }
        } else {
            return None;
        }
    }

    cache.table.insert(
        hash,
        CacheLine {
            depth,
            play: move_best.clone(),
            h: h_best,
        },
    );
    Some((move_best, h_best))
}

fn heuristic(game: &Game, b1: &Board, b2: &Board) -> isize {
    let walls = game.walls_left[game.player.as_index()];
    let coeff = if walls > 0 { 1 } else { 1 };

    coeff * _heuristic(game, game.player, b1) - _heuristic(game, game.player.opponent(), b2)
}

fn _heuristic(game: &Game, player: Player, board: &Board) -> isize {
    let pos = game.board.player_position(player);
    let tile = board.tiles[pos.y][pos.x];

    let dis = match tile {
        Tile::Invalid => {
            println!("\n!!!\nERROR(_heuristic:199)\n!!!\n");
            println!("{:?}", board);
            return 0;
        }
        Tile::Valid(_, dis) => dis,
    };
    if dis == 0 {
        return INF;
    }

    let mut h: isize = 0;

    h += (10.0 - dis as f64).max(0.0).sqrt().sqrt() as isize;
    h -= dis as isize * 10;

    h += game.walls_left[player.as_index()] as isize * 20;

    fn ahead_black(y: usize, pos_y: usize) -> bool {
        y < pos_y
    }
    fn ahead_white(y: usize, pos_y: usize) -> bool {
        y >= pos_y
    }
    let y_ahead = match player {
        Player::Black => ahead_black,
        Player::White => ahead_white,
    };
    let sign: isize = match player {
        Player::Black => -1,
        Player::White => 1,
    };

    for y in 0..WALL_GRID_HEIGHT {
        for x in 0..WALL_GRID_WIDTH {
            match game.board.walls.0[x][y] {
                None => {}
                Some(WallOrientation::Horizontal) => {
                    let dx = x as isize - pos.x as isize;
                    if y_ahead(y, pos.y) {
                        h += if dx.abs() <= 1 { -5 } else { -3 }
                    } else {
                        h += y as isize * sign;
                    }
                }
                Some(WallOrientation::Vertical) => {
                    let dx = x as isize - pos.x as isize;
                    if y_ahead(y, pos.y) {
                        h += if dx.abs() <= 1 { 0 } else { -1 }
                    } else {
                        // h += if dx.abs() <= 1 { 0 } else { 0 }
                    }
                }
            }
        }
    }

    h
}

use std::time::{Duration, Instant};

use crate::{
    bot::dedi::walls::{Tile, get_board, get_wall_moves},
    data_model::{Game, PIECE_GRID_HEIGHT, Player, PlayerMove},
    game_logic::{
        all_move_piece_moves, execute_move_unchecked,
        is_move_piece_legal_with_players_at_positions, new_position_after_move_piece_unchecked,
    },
};

pub const INF: isize = isize::MAX - 1;

pub fn minimax_iterative(game: &Game, duration: Duration) -> Option<PlayerMove> {
    let deadline = Some(Instant::now() + duration);
    let mut depth = 1;
    let mut best_move: Option<PlayerMove> = None;
    loop {
        if let Some((_move, h)) = minimax(game, depth, deadline) {
            println!("Found {:?} at level {:?} with h={:?}", _move, depth, h);
            best_move = _move;
            depth += 1;
        } else {
            break;
        }
    }

    best_move
}

pub fn minimax(
    game: &Game,
    depth: usize,
    deadline: Option<Instant>,
) -> Option<(Option<PlayerMove>, isize)> {
    _minimax(
        game,
        depth,
        -INF,
        INF,
        Tile::Invalid, // Tile::Invalid is ok as long as depth > 0
        Tile::Invalid,
        deadline,
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
    alpha: isize,
    beta: isize,
    tile_p1: Tile,
    tile_p2: Tile,
    deadline: Option<Instant>,
) -> Option<(Option<PlayerMove>, isize)> {
    if deadline.is_some_and(|deadline| Instant::now() > deadline) {
        return None;
    }
    if depth <= 0 {
        let h = heuristic(game, tile_p1, tile_p2);
        return Some((None, h));
    }
    let mut moves: Vec<(PlayerMove, Tile, Tile)> = Vec::new();

    let p1 = game.player;
    let p2 = game.player.opponent();
    let pos_p1 = game.board.player_position(game.player);
    let pos_p2 = game.board.player_position(game.player.opponent());

    if pos_p1.y == target(p1) {
        return Some((None, INF));
    }
    if pos_p2.y == target(p2) {
        return Some((None, -INF));
    }

    let board_p1 = get_board(&game, p1);
    let board_p2 = get_board(&game, p2);

    for move_piece in all_move_piece_moves(pos_p1, pos_p2) {
        let legal = is_move_piece_legal_with_players_at_positions(
            &game.board.walls,
            pos_p1,
            pos_p2,
            &move_piece,
        );

        if legal {
            let p = new_position_after_move_piece_unchecked(pos_p1, &move_piece, pos_p2);
            let t1 = board_p1.tiles[p.y][p.x];
            let t2 = board_p2.tiles[pos_p2.y][pos_p2.x];
            moves.push((PlayerMove::MovePiece(move_piece), t1, t2));
        }
    }

    for move_wall in get_wall_moves(game, &board_p1, &board_p2) {
        moves.push(move_wall);
    }

    if moves.len() == 0 {
        return Some((None, -INF));
    }

    let mut alpha = alpha;
    let mut h_best = -INF;
    let mut move_best: Option<PlayerMove> = None;

    for (_move, t1, t2) in moves {
        let game_next = execute_move_unchecked(game, &_move);
        if let Some((_, h_next)) = _minimax(&game_next, depth - 1, -beta, -alpha, t2, t1, deadline)
        {
            let h_inv = -h_next;

            if h_inv > h_best || move_best == None {
                h_best = h_inv;
                move_best = Some(_move);
            }
            alpha = isize::max(alpha, h_best);
            if alpha >= beta {
                break;
            }
        } else {
            return None;
        }
    }

    Some((move_best, h_best))
}

fn heuristic(game: &Game, t1: Tile, t2: Tile) -> isize {
    let p1_dis = match t1 {
        Tile::Invalid => return -INF,
        Tile::Valid(_, dis) => dis,
    };
    let p2_dis = match t2 {
        Tile::Invalid => return INF,
        Tile::Valid(_, dis) => dis,
    };
    if p1_dis == 0 {
        return INF;
    }
    if p2_dis == 0 {
        return -INF;
    }

    let mut h: isize = 0;

    h -= (p1_dis as isize) * 10;
    h += (p2_dis as isize) * 10;
    h += game.walls_left[game.player.as_index()] as isize;
    h -= game.walls_left[game.player.opponent().as_index()] as isize;

    h
}

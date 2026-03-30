use crate::{
    bot::{abe::heuristic_board_score, dedi::walls::get_wall_moves},
    data_model::{Game, Player, PlayerMove},
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_piece_legal_with_players_at_positions,
    },
};

pub const INF: isize = isize::MAX - 1;

pub fn minimax(game: &Game, depth: usize) -> (Option<PlayerMove>, isize) {
    let color = game.player;
    _minimax(game, depth, -INF, INF, color)
}

fn _minimax(
    game: &Game,
    depth: usize,
    alpha: isize,
    beta: isize,
    color: Player,
) -> (Option<PlayerMove>, isize) {
    let h = heuristic(game);
    if depth <= 0 || h == INF || h == -INF {
        return (None, h);
    }

    let mut moves: Vec<PlayerMove> = Vec::new();

    let p1 = game.board.player_position(game.player);
    let p2 = game.board.player_position(game.player.opponent());

    for move_piece in all_move_piece_moves(p1, p2) {
        let legal =
            is_move_piece_legal_with_players_at_positions(&game.board.walls, p1, p2, &move_piece);

        if legal {
            moves.push(PlayerMove::MovePiece(move_piece));
        }
    }

    for move_wall in get_wall_moves(game) {
        moves.push(move_wall);
    }

    if moves.len() == 0 {
        return (None, h);
    }

    let mut alpha = alpha;
    let mut h_best = -INF;
    let mut move_best: Option<PlayerMove> = None;

    for _move in moves {
        let game_next = execute_move_unchecked(game, &_move);
        let (_, h_next) = _minimax(&game_next, depth - 1, -beta, -alpha, color.opponent());
        let h_inv = -h_next;

        if h_inv > h_best || move_best == None {
            h_best = h_inv;
            move_best = Some(_move);
        }
        alpha = isize::max(alpha, h_best);
        if alpha >= beta {
            break;
        }
    }

    (move_best, h_best)
}

fn heuristic(game: &Game) -> isize {
    match game.player {
        Player::White => heuristic_board_score(game),
        Player::Black => -heuristic_board_score(game),
    }
}

use arrayvec::ArrayVec;
use rand::{Rng, rngs::ThreadRng, seq::IndexedRandom};

use crate::{
    bot::dedi::walls::{get_board, get_wall_moves, wall_collide},
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked_inplace,
        is_move_piece_legal_with_players_at_positions,
    },
};
use std::time::{Duration, Instant};

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

    let mut rng = rand::rng();
    let mut win_count = vec![0isize; legal_moves.len()];
    let mut iterations = 0;

    while Instant::now() < deadline {
        for (i, m) in legal_moves.iter().enumerate() {
            win_count[i] += simulate(game, &mut rng, m.clone(), &wall_moves);
        }
        iterations += 1;
    }

    let best_idx = win_count
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(i, _)| i)
        .expect("No legal moves");

    println!("{:?} / {:?}", win_count[best_idx], iterations);
    legal_moves[best_idx].clone()
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
    get_legal_piece_moves(game).chain(get_legal_wall_moves(game))
}

fn get_legal_piece_moves(game: &Game) -> impl Iterator<Item = PlayerMove> {
    let p1 = game.player;
    let p2 = game.player.opponent();
    let pos_p1 = game.board.player_position(p1);
    let pos_p2 = game.board.player_position(p2);

    all_move_piece_moves(pos_p1, pos_p2)
        .filter(|m| {
            is_move_piece_legal_with_players_at_positions(&game.board.walls, pos_p1, pos_p2, &m)
        })
        .map(|m| PlayerMove::MovePiece(m))
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

    for _ in 1..64 {
        let p1_pos = game.board.player_position(p1);
        if p1_pos.y == p1_target {
            return 1;
        }
        let p2_pos = game.board.player_position(p2);
        if p2_pos.y == p2_target {
            return -1;
        }

        let piece_moves: ArrayVec<_, 8> = get_legal_piece_moves(&game).collect();
        let m = if piece_moves.len() > 0 && rng.random_bool(0.8) {
            piece_moves.choose(rng).unwrap().clone()
        } else if wall_moves.len() == 0 {
            return 0;
        } else {
            let idx = rng.random_range(0..wall_moves.len());
            wall_moves.swap_remove(idx).clone()
        };

        execute_move_unchecked_inplace(&mut game, &m);
    }

    0
}

fn target(player: Player) -> usize {
    if player == Player::White {
        PIECE_GRID_HEIGHT - 1
    } else {
        0
    }
}

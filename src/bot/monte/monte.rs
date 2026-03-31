use arrayvec::ArrayVec;
use rand::{Rng, rngs::ThreadRng, seq::IndexedRandom};

use crate::{
    bot::dedi::walls::{Dir, get_board, get_wall_moves, wall_blocks, wall_collide},
    data_model::{
        Game, MovePiece, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation, WallPosition,
    },
    game_logic::execute_move_unchecked_inplace,
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
    get_legal_piece_moves(game)
        .into_iter()
        .chain(get_legal_wall_moves(game))
}

fn get_legal_piece_moves(game: &Game) -> ArrayVec<PlayerMove, 8> {
    let p1 = game.board.player_position(game.player);
    let p1 = (p1.x, p1.y);
    let p2 = game.board.player_position(game.player.opponent());
    let p2 = (p2.x, p2.y);
    let mut moves: ArrayVec<PlayerMove, 8> = ArrayVec::new();

    let allow = |xy: (usize, usize), dir: Dir| {
        dir.can_apply(xy) && !wall_blocks(&game.board.walls, xy.0 as isize, xy.1 as isize, dir)
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

        let piece_moves = get_legal_piece_moves(&game);
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

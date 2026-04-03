use std::ops::Range;

use crate::{
    bot::abe::alpha_beta::{WHITE_LOSES_BLACK_WINS, WHITE_WINS_BLACK_LOSES},
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, TOTAL_WALLS, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation,
    },
    l_p_a_star::Pathfinding,
};

#[derive(Default, Debug, Clone, Copy, clap_derive::ValueEnum)]
pub enum Heuristic {
    #[default]
    WallsBehind,
    Forward,
    Basic,
    DistanceOnly,
}

impl Heuristic {
    pub fn eval(&self, game: &Game, pathfinding: &mut Pathfinding, verbose: bool) -> isize {
        match self {
            Heuristic::WallsBehind => walls_behind(game, pathfinding, verbose),
            Heuristic::Forward => forward(game, pathfinding, verbose),
            Heuristic::Basic => basic(game, pathfinding, verbose),
            Heuristic::DistanceOnly => distance_only(game, pathfinding, verbose),
        }
    }
}

fn walls_behind(game: &Game, pathfinding: &mut Pathfinding, verbose: bool) -> isize {
    let black_pos = game.board.player_position(Player::Black);
    let black_distance = pathfinding
        .black
        .distance_to_goal(black_pos, &game.board.walls) as isize;
    if black_distance == 0 {
        return WHITE_LOSES_BLACK_WINS;
    }
    let white_pos = game.board.player_position(Player::White);
    let white_distance = pathfinding
        .white
        .distance_to_goal(white_pos, &game.board.walls) as isize;
    if white_distance == 0 {
        return WHITE_WINS_BLACK_LOSES;
    }
    let white_walls_left = game.walls_left[Player::White.as_index()] as isize;
    let black_walls_left = game.walls_left[Player::Black.as_index()] as isize;
    let white_wall_score = white_walls_left * (50 + black_distance * 5);
    let black_wall_score = black_walls_left * (50 + white_distance * 5);

    let distance_score = (black_distance - white_distance) * 100;
    let wall_score = white_wall_score - black_wall_score;

    let walls_behind = |range: Range<usize>| {
        range
            .map(|y| {
                usize::max(
                    (0..WALL_GRID_WIDTH)
                        .step_by(2)
                        .filter(|x| {
                            game.board.walls.wall_at(
                                WallOrientation::Horizontal,
                                *x as isize,
                                y as isize,
                            )
                        })
                        .count(),
                    (1..WALL_GRID_WIDTH)
                        .step_by(2)
                        .filter(|x| {
                            game.board.walls.wall_at(
                                WallOrientation::Horizontal,
                                *x as isize,
                                y as isize,
                            )
                        })
                        .count(),
                )
            })
            .max()
            .unwrap_or_default() as isize
    };
    let white_walls_behind = walls_behind(0..white_pos.y);
    let black_walls_behind = walls_behind(black_pos.y..WALL_GRID_HEIGHT);

    let walls_left =
        game.walls_left[Player::White.as_index()] + game.walls_left[Player::Black.as_index()];
    let walls_behind_score =
        (white_walls_behind - black_walls_behind) * (50 * walls_left / TOTAL_WALLS) as isize;

    if verbose {
        println!(
            "wall_score: {wall_score}, distance_score: {distance_score}, walls_behind_score: {walls_behind_score}, "
        );
    }

    wall_score + distance_score + walls_behind_score
}

fn forward(game: &Game, _pathfinding: &mut Pathfinding, _verbose: bool) -> isize {
    let black = game.board.player_position(Player::Black).y as isize;
    let white =
        PIECE_GRID_HEIGHT as isize - 1 - (game.board.player_position(Player::White).y as isize);
    100 * (black - white)
}

fn basic(game: &Game, pathfinding: &mut Pathfinding, _verbose: bool) -> isize {
    let black_distance = pathfinding
        .black
        .distance_to_goal(game.board.player_position(Player::Black), &game.board.walls)
        as isize;
    if black_distance == 0 {
        return WHITE_LOSES_BLACK_WINS;
    }
    let white_distance = pathfinding
        .white
        .distance_to_goal(game.board.player_position(Player::White), &game.board.walls)
        as isize;
    if white_distance == 0 {
        return WHITE_WINS_BLACK_LOSES;
    }
    let white_walls_left = game.walls_left[Player::White.as_index()] as isize;
    let black_walls_left = game.walls_left[Player::Black.as_index()] as isize;
    let distance_score = black_distance - white_distance;
    let wall_score = white_walls_left - black_walls_left;
    let total_walls_played = TOTAL_WALLS
        - game.walls_left[Player::White.as_index()]
        - game.walls_left[Player::Black.as_index()];
    let wall_progress = total_walls_played as f32 / TOTAL_WALLS as f32;
    let wall_value = 75.0 - 50.0 * wall_progress;
    let (distance_priority, wall_priority) = (100, wall_value);

    distance_priority * distance_score + (wall_priority * wall_score as f32) as isize
}

fn distance_only(game: &Game, pathfinding: &mut Pathfinding, _verbose: bool) -> isize {
    let black_distance = pathfinding
        .black
        .distance_to_goal(game.board.player_position(Player::Black), &game.board.walls)
        as isize;
    if black_distance == 0 {
        return WHITE_LOSES_BLACK_WINS;
    }
    let white_distance = pathfinding
        .white
        .distance_to_goal(game.board.player_position(Player::White), &game.board.walls)
        as isize;
    if white_distance == 0 {
        return WHITE_WINS_BLACK_LOSES;
    }
    (black_distance - white_distance) * 100
}

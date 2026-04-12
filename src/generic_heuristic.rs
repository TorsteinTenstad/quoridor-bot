use crate::data_model::{Game, PIECE_GRID_HEIGHT, Player, TOTAL_WALLS};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenericHeuristicWeights {
    pub distance: f32,
    pub walls_left: f32,
    pub opponent_distance_x_walls_left: f32,
    pub manhattan_distance_x_wall_progress: f32,
    pub walls_left_x_wall_progress: f32,
}

impl GenericHeuristicWeights {
    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.distance,
            self.walls_left,
            self.opponent_distance_x_walls_left,
            self.manhattan_distance_x_wall_progress,
            self.walls_left_x_wall_progress,
        ]
    }
    pub fn from_slice(slice: &[f32]) -> Option<Self> {
        let mut iter = slice.iter();
        Some(Self {
            distance: *iter.next()?,
            walls_left: *iter.next()?,
            opponent_distance_x_walls_left: *iter.next()?,
            manhattan_distance_x_wall_progress: *iter.next()?,
            walls_left_x_wall_progress: *iter.next()?,
        })
    }
}

pub fn generic_heuristic(
    game: &Game,
    weights: &GenericHeuristicWeights,
    distance: u8,
    opponent_distance: u8,
) -> isize {
    let total_walls_played = TOTAL_WALLS
        - game.walls_left[Player::White.as_index()]
        - game.walls_left[Player::Black.as_index()];
    let wall_progress = total_walls_played as f32 / TOTAL_WALLS as f32;
    let manhattan_distance = match game.player {
        Player::Black => game.board.player_position(Player::Black).y,
        Player::White => PIECE_GRID_HEIGHT - 1 - game.board.player_position(Player::White).y,
    } as f32;

    let distance = distance as f32;
    let walls_left = game.walls_left[game.player.as_index()] as f32;
    let opponent_distance_x_walls_left = opponent_distance as f32 * walls_left;
    let manhattan_distance_x_wall_progress = manhattan_distance * wall_progress;
    let walls_left_x_wall_progress = walls_left * wall_progress;

    (distance * weights.distance
        + walls_left * weights.walls_left
        + opponent_distance_x_walls_left * weights.opponent_distance_x_walls_left
        + manhattan_distance_x_wall_progress * weights.manhattan_distance_x_wall_progress
        + walls_left_x_wall_progress * weights.walls_left_x_wall_progress) as isize
}

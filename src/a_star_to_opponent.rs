use crate::data_model::{Board, Direction, PiecePosition, Player, Walls};
use crate::game_logic::{
    is_move_direction_legal_with_player_at_position, new_position_after_direction_unchecked,
};
use crate::priority_queue::PriorityQueue;
use std::collections::HashMap;

pub fn heuristic(pos: &PiecePosition, opponent_pos: &PiecePosition) -> usize {
    let dx = usize::abs_diff(pos.x, opponent_pos.x);
    let dy = usize::abs_diff(pos.y, opponent_pos.y);
    dx + dy
}

pub fn a_star_to_opponent(board: &Board, player: Player) -> Option<Vec<PiecePosition>> {
    let start = board.player_position(player).clone();
    let mut open_set = PriorityQueue::new();
    let mut came_from = HashMap::<PiecePosition, PiecePosition>::new();
    let mut g_score = HashMap::<PiecePosition, usize>::new();
    let mut f_score = HashMap::<PiecePosition, usize>::new();
    g_score.insert(start.clone(), 0);
    let h = heuristic(&start, board.player_position(player.opponent()));
    f_score.insert(start.clone(), h);
    open_set.insert(h, start.clone());

    while let Some((_, current)) = open_set.pop() {
        if heuristic(&current, board.player_position(player.opponent())) == 0 {
            return Some(reconstruct_path(&came_from, &current));
        }
        for neighbor in neighbors(&board.walls, &current) {
            let tentative_g_score = g_score[&current] + 1;
            if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&usize::MAX) {
                came_from.insert(neighbor.clone(), current.clone());
                g_score.insert(neighbor.clone(), tentative_g_score);
                let f = tentative_g_score
                    + heuristic(&neighbor, board.player_position(player.opponent()));
                f_score.insert(neighbor.clone(), f);

                open_set.insert(f, neighbor.clone());
            }
        }
    }

    None
}

fn reconstruct_path(
    came_from: &HashMap<PiecePosition, PiecePosition>,
    current: &PiecePosition,
) -> Vec<PiecePosition> {
    let mut total_path = Vec::new();
    let mut current = current;
    while let Some(next) = came_from.get(current) {
        total_path.push(current.clone());
        current = next;
    }
    total_path.reverse();
    total_path
}

pub fn neighbors(
    walls: &Walls,
    player_position: &PiecePosition,
) -> impl Iterator<Item = PiecePosition> {
    Direction::iter()
        .filter(|direction| {
            is_move_direction_legal_with_player_at_position(walls, player_position, direction)
        })
        .map(|direction| new_position_after_direction_unchecked(player_position, direction))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_model::Game;

    #[test]
    fn distance_between_players() {
        let mut game = Game::new();
        game.board.player_positions[0] = PiecePosition::new(0, 0);
        game.board.player_positions[1] = PiecePosition::new(1, 0);
        let path = a_star_to_opponent(&game.board, Player::White);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 1);
    }
}

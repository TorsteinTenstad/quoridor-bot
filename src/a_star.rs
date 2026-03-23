use crate::data_model::{Board, MovePiece, PIECE_GRID_HEIGHT, PiecePosition, Player};
use crate::game_logic::{
    is_move_piece_legal_with_player_at_position, new_position_after_move_piece_unchecked,
};
use crate::priority_queue::PriorityQueue;
use std::collections::HashMap;

pub fn heuristic(pos: &PiecePosition, player: Player) -> usize {
    match player {
        Player::White => PIECE_GRID_HEIGHT - 1 - pos.y(),
        Player::Black => pos.y(),
    }
}

pub fn a_star(board: &Board, player: Player) -> Option<Vec<PiecePosition>> {
    let start = board.player_position(player).clone();
    let mut open_set = PriorityQueue::new();
    let mut came_from = HashMap::<PiecePosition, PiecePosition>::new();
    let mut g_score = HashMap::<PiecePosition, usize>::new();
    let mut f_score = HashMap::<PiecePosition, usize>::new();
    g_score.insert(start.clone(), 0);
    let h = heuristic(&start, player);
    f_score.insert(start.clone(), h);
    open_set.insert(h, start.clone());

    while let Some((_, current)) = open_set.pop() {
        if heuristic(&current, player) == 0 {
            return Some(reconstruct_path(&came_from, &current));
        }
        for neighbor in neighbors(board, player, &current) {
            let tentative_g_score = g_score[&current] + 1;
            if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&usize::MAX) {
                came_from.insert(neighbor.clone(), current.clone());
                g_score.insert(neighbor.clone(), tentative_g_score);
                let f = tentative_g_score + heuristic(&neighbor, player);
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

fn neighbors(board: &Board, player: Player, player_position: &PiecePosition) -> Vec<PiecePosition> {
    MovePiece::iter()
        .filter_map(|move_piece| {
            is_move_piece_legal_with_player_at_position(board, player, player_position, &move_piece)
                .then(|| {
                    new_position_after_move_piece_unchecked(
                        player_position,
                        &move_piece,
                        board.player_position(player.opponent()),
                    )
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_model::{Game, WallOrientation};

    #[test]
    fn single_wall_test() {
        let mut game = Game::new();
        game.board.walls[3][2] = Some(WallOrientation::Horizontal);
        let path = a_star(&game.board, Player::White);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(
            path,
            vec![
                PiecePosition::new(4, 1),
                PiecePosition::new(4, 2),
                PiecePosition::new(5, 2),
                PiecePosition::new(5, 3),
                PiecePosition::new(5, 4),
                PiecePosition::new(5, 5),
                PiecePosition::new(5, 6),
                PiecePosition::new(5, 7),
                PiecePosition::new(5, 8),
            ]
        );
    }

    #[test]
    fn complex_wall_test() {
        let mut game = Game::new();
        game.board.player_positions[Player::White.as_index()] = PiecePosition::new(4, 4);
        game.board.player_positions[Player::Black.as_index()] = PiecePosition::new(3, 4);
        game.board.walls[2][3] = Some(WallOrientation::Vertical);
        game.board.walls[3][3] = Some(WallOrientation::Vertical);
        game.board.walls[2][5] = Some(WallOrientation::Vertical);
        game.board.walls[4][3] = Some(WallOrientation::Horizontal);
        game.board.walls[4][4] = Some(WallOrientation::Horizontal);
        game.board.walls[5][5] = Some(WallOrientation::Vertical);
        let path = a_star(&game.board, Player::White);
        assert!(path.is_some());
    }

    #[test]
    fn on_goal_test() {
        let mut game = Game::new();
        game.board.player_positions[0] = PiecePosition::new(4, 8);
        let path = a_star(&game.board, Player::White);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 0);
    }
}

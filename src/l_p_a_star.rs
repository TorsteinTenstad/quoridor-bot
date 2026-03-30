use crate::{
    a_star_to_opponent::neighbors,
    data_model::{
        Board, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, PiecePosition, Player, PlayerMove,
        WallOrientation, WallPosition, Walls,
    },
    priority_queue::PriorityQueue,
};
use std::array::from_fn;

#[derive(Debug, Clone)]
pub struct Pathfinding {
    pub white: LPAStar,
    pub black: LPAStar,
}

impl Pathfinding {
    pub fn new(board: &Board) -> Self {
        Self {
            white: LPAStar::new(
                Player::White,
                board.player_position(Player::White),
                board.walls.clone(),
            ),
            black: LPAStar::new(
                Player::Black,
                board.player_position(Player::Black),
                board.walls.clone(),
            ),
        }
    }
    pub fn any_blocked(&mut self, board: &Board) -> bool {
        self.white
            .distance_to_goal(board.player_position(Player::White))
            == u16::MAX
            || self
                .black
                .distance_to_goal(board.player_position(Player::Black))
                == u16::MAX
    }
    pub fn clone_with_move(&self, new_board: &Board, m: &PlayerMove) -> Self {
        let mut clone = self.clone();
        match m {
            PlayerMove::MovePiece(_) => (),
            PlayerMove::PlaceWall {
                orientation,
                position,
            } => {
                clone.white.place_wall(
                    position,
                    *orientation,
                    new_board.player_position(Player::White),
                );
                clone.black.place_wall(
                    position,
                    *orientation,
                    new_board.player_position(Player::Black),
                );
            }
        }
        clone
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Estimates {
    g: u16,
    rhs: u16,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Key(u16, u16);

pub fn heuristic(pos: &PiecePosition, goal: &PiecePosition) -> u16 {
    (usize::abs_diff(pos.x, goal.x) + usize::abs_diff(pos.y, goal.y)) as u16
}

pub fn start_y(player: Player) -> usize {
    match player {
        Player::White => PIECE_GRID_HEIGHT - 1,
        Player::Black => 0,
    }
}

pub fn is_start(pos: &PiecePosition, player: Player) -> bool {
    pos.y == start_y(player)
}

#[derive(Debug, Clone)]
pub struct LPAStar {
    board: [[Estimates; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
    walls: Walls,
    player: Player,
    queue: PriorityQueue<Key, PiecePosition>,
}

impl LPAStar {
    pub fn new(player: Player, goal: &PiecePosition, walls: Walls) -> Self {
        let start_estimates = Estimates {
            g: u16::MAX,
            rhs: 0,
        };
        let default_estimates = Estimates {
            g: u16::MAX,
            rhs: u16::MAX,
        };
        let board = from_fn(|y| {
            from_fn(|x| {
                if is_start(&PiecePosition::new(x, y), player) {
                    start_estimates.clone()
                } else {
                    default_estimates.clone()
                }
            })
        });
        let mut queue = PriorityQueue::<Key, PiecePosition>::default();
        for start_x in 0..PIECE_GRID_WIDTH {
            let pos = PiecePosition::new(start_x, start_y(player));
            queue.insert(
                calculate_key_of_estimates(&start_estimates, &pos, goal),
                pos,
            );
        }
        Self {
            board,
            walls,
            player,
            queue,
        }
    }
    fn place_wall(
        &mut self,
        pos: &WallPosition,
        orientation: WallOrientation,
        goal: &PiecePosition,
    ) {
        self.walls.0[pos.x][pos.y] = Some(orientation);
        for dx in 0..2 {
            for dy in 0..2 {
                self.update_node(&PiecePosition::new(pos.x + dx, pos.y + dy), goal);
            }
        }
    }
    pub fn distance_to_goal(&mut self, goal: &PiecePosition) -> u16 {
        self.compute_shortest_path(goal);
        let estimates = &self.board[goal.y][goal.x];
        estimates.g
    }
    fn compute_shortest_path(&mut self, goal: &PiecePosition) {
        loop {
            let Some((top_key, top_pos)) = self.queue.pop() else {
                break;
            };
            if (top_key >= self.calculate_key(goal, goal))
                && !self.is_inconsistent(goal)
                && (0..PIECE_GRID_WIDTH)
                    .all(|x| !self.is_inconsistent(&PiecePosition::new(x, start_y(self.player))))
            {
                break;
            }
            let estimates = &mut self.board[top_pos.y][top_pos.x];
            if estimates.g > estimates.rhs {
                estimates.g = estimates.rhs;
            } else {
                estimates.g = u16::MAX;
                self.update_node(&top_pos, goal);
            }
            // TODO: collect can be optimized out:
            for neighbor in neighbors(&self.walls, &top_pos).collect::<Vec<_>>() {
                self.update_node(&neighbor, goal);
            }
        }
    }
    fn update_node(&mut self, pos: &PiecePosition, goal: &PiecePosition) {
        if !is_start(pos, self.player) {
            let new_rhs = neighbors(&self.walls, pos)
                .map(|neigbor_pos| self.board[neigbor_pos.y][neigbor_pos.x].g.saturating_add(1))
                .min()
                .unwrap_or(u16::MAX);
            self.board[pos.y][pos.x].rhs = new_rhs;
            if self.queue.contains(pos) {
                self.queue.remove(pos);
            }
            if self.is_inconsistent(pos) {
                self.queue
                    .insert(self.calculate_key(pos, goal), pos.clone());
            }
        }
    }

    fn is_inconsistent(&self, pos: &PiecePosition) -> bool {
        let estimates = &self.board[pos.y][pos.x];
        estimates.g != estimates.rhs
    }

    fn calculate_key(&self, pos: &PiecePosition, goal: &PiecePosition) -> Key {
        let estimates = &self.board[pos.y][pos.x];
        calculate_key_of_estimates(estimates, pos, goal)
    }
}

fn calculate_key_of_estimates(
    estimates: &Estimates,
    pos: &PiecePosition,
    goal: &PiecePosition,
) -> Key {
    let k2 = u16::min(estimates.g, estimates.rhs);
    let k1 = k2 + heuristic(pos, goal);
    Key(k1, k2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_model::{Direction, Game, MovePiece, WallOrientation},
        game_logic::execute_move_unchecked,
    };

    #[test]
    fn empty_board() {
        let board = Board::new();
        let mut pathfinding = Pathfinding::new(&board);
        let a = pathfinding
            .black
            .distance_to_goal(&PiecePosition::new(0, 1));
        for row in &pathfinding.black.board {
            for cell in row {
                print!("({:5} {:5})", cell.g, cell.rhs);
            }
            println!();
        }
        assert_eq!(a, 1);
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 0)),
            0
        );
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5)),
            5
        );
    }
    #[test]
    fn walls() {
        let mut board = Board::new();
        board.walls.0[0][2] = Some(WallOrientation::Horizontal);
        board.walls.0[2][2] = Some(WallOrientation::Horizontal);
        let mut pathfinding = Pathfinding::new(&board);
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5)),
            9
        );
    }
    #[test]
    #[ntest::timeout(1000)]
    fn blocked() {
        let mut game = Game::new();
        game.board.walls.0[0][1] = Some(WallOrientation::Horizontal);
        game.board.walls.0[2][1] = Some(WallOrientation::Horizontal);
        game.board.walls.0[4][1] = Some(WallOrientation::Horizontal);
        game.board.walls.0[5][0] = Some(WallOrientation::Vertical);
        let mut pathfinding = Pathfinding::new(&game.board);
        assert_eq!(
            pathfinding
                .white
                .distance_to_goal(&PiecePosition::new(0, 0)),
            u16::MAX
        );
        assert!(pathfinding.any_blocked(&game.board))
    }
    #[test]
    #[ntest::timeout(1000)]
    fn iterative() {
        let mut game = Game::new();
        game.board.walls.0[0][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[2][2] = Some(WallOrientation::Horizontal);
        let mut pathfinding = Pathfinding::new(&game.board);
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5)),
            9
        );
        pathfinding.black.place_wall(
            &WallPosition { x: 4, y: 2 },
            WallOrientation::Horizontal,
            &PiecePosition::new(0, 5),
        );
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5)),
            11
        );
        let m = &PlayerMove::MovePiece(MovePiece {
            direction: Direction::Right,
            direction_on_collision: Direction::Right,
        });
        assert_eq!(
            pathfinding
                .clone_with_move(&execute_move_unchecked(&game, m).board, m)
                .black
                .distance_to_goal(&PiecePosition::new(1, 5)),
            10
        )
    }
}

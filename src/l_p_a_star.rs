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
            white: LPAStar::new(Player::White, board.player_position(Player::White)),
            black: LPAStar::new(Player::Black, board.player_position(Player::Black)),
        }
    }
    pub fn any_blocked(&mut self, board: &Board) -> bool {
        self.white
            .distance_to_goal(board.player_position(Player::White), &board.walls)
            == u16::MAX
            || self
                .black
                .distance_to_goal(board.player_position(Player::Black), &board.walls)
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
                    &new_board.walls,
                );
                clone.black.place_wall(
                    position,
                    *orientation,
                    new_board.player_position(Player::Black),
                    &new_board.walls,
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
    player: Player,
    queue: PriorityQueue<Key, PiecePosition>,
}

impl LPAStar {
    pub fn new(player: Player, goal: &PiecePosition) -> Self {
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
            player,
            queue,
        }
    }
    fn place_wall(
        &mut self,
        pos: &WallPosition,
        _orientation: WallOrientation,
        goal: &PiecePosition,
        walls: &Walls,
    ) {
        for dx in 0..2 {
            for dy in 0..2 {
                let p = PiecePosition::new(pos.x + dx, pos.y + dy);
                self.update_node(&p, goal, walls);
            }
        }
    }
    pub fn distance_to_goal(&mut self, goal: &PiecePosition, walls: &Walls) -> u16 {
        self.compute_shortest_path(goal, walls);
        let estimates = &self.board[goal.y][goal.x];
        estimates.g
    }
    fn compute_shortest_path(&mut self, goal: &PiecePosition, walls: &Walls) {
        loop {
            let Some(top_key) = self.queue.peek().map(|x| x.0) else {
                break;
            };
            let goal_key = self.calculate_key(goal, goal);
            let goal_inconsistent = self.is_inconsistent(goal);

            let any_start_inconsistent = (0..PIECE_GRID_WIDTH)
                .any(|x| self.is_inconsistent(&PiecePosition::new(x, start_y(self.player))));

            if !goal_inconsistent && !any_start_inconsistent && top_key >= goal_key {
                break;
            }

            let Some((_, top_pos)) = self.queue.pop() else {
                break;
            };
            let estimates = &mut self.board[top_pos.y][top_pos.x];
            if estimates.g > estimates.rhs {
                estimates.g = estimates.rhs;
            } else {
                estimates.g = u16::MAX;
                self.update_node(&top_pos, goal, walls);
            }
            for neighbor in neighbors(walls, &top_pos) {
                self.update_node(&neighbor, goal, walls);
            }
        }
    }
    fn update_node(&mut self, pos: &PiecePosition, goal: &PiecePosition, walls: &Walls) {
        if !is_start(pos, self.player) {
            let new_rhs = neighbors(walls, pos)
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

pub fn dump(state: &LPAStar, walls: &Walls) {
    for (y, row) in state.board.iter().enumerate() {
        let y = y as isize;
        for (x, cell) in row.iter().enumerate() {
            let x = x as isize;
            let f = |n| {
                if n == u16::MAX {
                    "xx".into()
                } else {
                    format!("{:2}", n)
                }
            };
            let w = if walls.wall_at(WallOrientation::Vertical, x, y)
                || walls.wall_at(WallOrientation::Vertical, x, y - 1)
            {
                '|'
            } else {
                ' '
            };
            print!("({} {}){}", f(cell.g), f(cell.rhs), w);
        }
        println!();
        for (x, _cell) in row.iter().enumerate() {
            let x = x as isize;
            let w = if walls.wall_at(WallOrientation::Horizontal, x, y)
                || walls.wall_at(WallOrientation::Horizontal, x - 1, y)
            {
                "-------"
            } else {
                "       "
            };
            print!("{} ", w);
        }
        println!();
    }
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
            .distance_to_goal(&PiecePosition::new(0, 1), &board.walls);
        assert_eq!(a, 1);
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 0), &board.walls),
            0
        );
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5), &board.walls),
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
                .distance_to_goal(&PiecePosition::new(0, 5), &board.walls),
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
                .distance_to_goal(&PiecePosition::new(0, 0), &game.board.walls),
            u16::MAX
        );
        assert!(pathfinding.any_blocked(&game.board))
    }
    #[test]
    #[ntest::timeout(1000)]
    fn blocked_iterative() {
        let mut game = Game::new();
        game.board.walls.0[2][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[2][3] = Some(WallOrientation::Horizontal);
        game.board.walls.0[2][4] = Some(WallOrientation::Horizontal);
        game.board.walls.0[1][3] = Some(WallOrientation::Vertical);
        game.board.player_positions[Player::Black.as_index()] = PiecePosition::new(2, 3);
        game.board.player_positions[Player::White.as_index()] = PiecePosition::new(2, 4);
        let mut pathfinding = Pathfinding::new(&game.board);
        assert!(!pathfinding.any_blocked(&game.board));
        let m = &PlayerMove::PlaceWall {
            orientation: WallOrientation::Vertical,
            position: WallPosition { x: 3, y: 3 },
        };
        let new_board = &execute_move_unchecked(&game, m).board;
        let mut pathfinding = pathfinding.clone_with_move(new_board, m);

        assert!(pathfinding.any_blocked(new_board));
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
                .distance_to_goal(&PiecePosition::new(0, 5), &game.board.walls),
            9
        );
        let m = &PlayerMove::PlaceWall {
            orientation: WallOrientation::Horizontal,
            position: WallPosition { x: 4, y: 2 },
        };
        let new_board = &execute_move_unchecked(&game, m).board;
        let mut pathfinding = pathfinding.clone_with_move(new_board, m);
        assert_eq!(
            pathfinding
                .black
                .distance_to_goal(&PiecePosition::new(0, 5), &new_board.walls),
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
                .distance_to_goal(&PiecePosition::new(1, 5), &game.board.walls),
            10
        )
    }

    fn game() -> Game {
        let mut game = Game::new();
        game.board.walls.0[0][1] = Some(WallOrientation::Horizontal);
        game.board.walls.0[2][1] = Some(WallOrientation::Horizontal);
        game.board.walls.0[1][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[3][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[5][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[7][2] = Some(WallOrientation::Horizontal);
        game.board.walls.0[3][3] = Some(WallOrientation::Vertical);
        game.board.walls.0[4][4] = Some(WallOrientation::Horizontal);
        game.board.walls.0[6][4] = Some(WallOrientation::Horizontal);
        game.board.walls.0[3][5] = Some(WallOrientation::Vertical);
        game.board.walls.0[5][5] = Some(WallOrientation::Horizontal);
        game.board.walls.0[7][5] = Some(WallOrientation::Vertical);
        game.board.walls.0[6][6] = Some(WallOrientation::Vertical);
        game.board.walls.0[6][7] = Some(WallOrientation::Horizontal);
        game.board.player_positions[Player::White.as_index()] = PiecePosition { x: 6, y: 4 };
        game
    }

    #[test]
    #[ntest::timeout(1000)]
    fn iterative_complex() {
        let game = game();
        let mut pathfinding = Pathfinding::new(&game.board);
        assert_eq!(
            pathfinding
                .white
                .distance_to_goal(game.board.player_position(Player::White), &game.board.walls),
            6
        );
        let m = &PlayerMove::PlaceWall {
            orientation: WallOrientation::Horizontal,
            position: WallPosition { x: 7, y: 3 },
        };
        let game = execute_move_unchecked(&game, m);
        let mut pathfinding = pathfinding.clone_with_move(&game.board, m);
        assert_eq!(
            pathfinding
                .white
                .distance_to_goal(game.board.player_position(Player::White), &game.board.walls),
            6
        );
        let m = &PlayerMove::PlaceWall {
            orientation: WallOrientation::Horizontal,
            position: WallPosition { x: 7, y: 6 },
        };
        let game = execute_move_unchecked(&game, m);
        println!("---\n\n\n\n\n\n\n\n\n\n\n\n\n\n---");
        let mut pathfinding = pathfinding.clone_with_move(&game.board, m);
        let d = pathfinding
            .white
            .distance_to_goal(game.board.player_position(Player::White), &game.board.walls);
        assert_eq!(d, u16::MAX);
    }
}

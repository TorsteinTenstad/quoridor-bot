use crate::{
    bot::carlo::bfs::{Bfs, PathBlock},
    data_model::{
        Game, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_piece_legal_with_players_at_positions,
    },
};
use std::fmt::Debug;

#[derive(Clone)]
pub struct Board {
    pub game: Game,
    bfs_white: Bfs,
    bfs_black: Bfs,
    wall_moves: [[[bool; 2]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // for row in self.path {
        //     for square in row {
        //         let _ = write!(f, "{:3}{:?} ", square.1, square.0);
        //     }
        //     let _ = writeln!(f);
        // }
        Ok(())
    }
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        let mut game_white = game.clone();
        let mut game_black = game.clone();
        game_white.player = Player::White;
        game_black.player = Player::Black;

        let mut board = Board {
            game: game.clone(),
            bfs_white: Bfs::from(&game_white),
            bfs_black: Bfs::from(&game_black),
            wall_moves: [[[true, true]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
        };

        for x in 0..WALL_GRID_WIDTH {
            for y in 0..WALL_GRID_HEIGHT {
                match board.game.board.walls.0[x][y] {
                    Some(orientation) => board.place_wall(x, y, orientation),
                    None => {}
                }
            }
        }

        board
    }
}

impl Board {
    pub fn play_move(&mut self, m: PlayerMove) {
        self.game = execute_move_unchecked(&self.game, &m);
        match m {
            PlayerMove::PlaceWall {
                orientation,
                position,
            } => {
                self.place_wall(position.x, position.y, orientation);

                self.bfs_white
                    .recalculate_bfs(&self.game, position.x, position.y, orientation);
                self.bfs_black
                    .recalculate_bfs(&self.game, position.x, position.y, orientation);
            }
            _ => {}
        }
    }

    pub fn moves(&self) -> impl Iterator<Item = PlayerMove> {
        let player_moves = {
            let p1 = self.game.board.player_position(self.game.player);
            let p2 = self.game.board.player_position(self.game.player.opponent());

            all_move_piece_moves(p1, p2)
                .filter(|m| {
                    is_move_piece_legal_with_players_at_positions(&self.game.board.walls, p1, p2, m)
                })
                .map(|m| PlayerMove::MovePiece(m))
        };

        // TODO: better wall_moves conditional
        let takes = if self.game.walls_left[self.game.player.as_index()] > 0 {
            1000000
        } else {
            0
        };

        let wall_moves =
            self.wall_moves
                .iter()
                .take(takes)
                .enumerate()
                .flat_map(move |(y, col)| {
                    col.iter().enumerate().flat_map(move |(x, ors)| {
                        ors.iter().enumerate().filter_map(move |(orient, allowed)| {
                            if !allowed {
                                return None;
                            }
                            let orientation =
                                [WallOrientation::Horizontal, WallOrientation::Vertical][orient];

                            if !self.valid_wall(x, y, orientation) {
                                return None;
                            }

                            Some(PlayerMove::PlaceWall {
                                orientation,
                                position: WallPosition { x, y },
                            })
                        })
                    })
                });

        player_moves.chain(wall_moves)
    }

    pub fn valid_wall(&self, x: usize, y: usize, orientation: WallOrientation) -> bool {
        let mut board = self.clone();
        board.place_wall(x, y, orientation);

        board
            .bfs_white
            .recalculate_bfs(&board.game, x, y, orientation);
        board
            .bfs_black
            .recalculate_bfs(&board.game, x, y, orientation);

        let white_pos = board.game.board.player_position(Player::White);
        let black_pos = board.game.board.player_position(Player::Black);

        match (
            board.bfs_white.path[white_pos.y][white_pos.x],
            board.bfs_black.path[black_pos.y][black_pos.x],
        ) {
            ((PathBlock::Unreachable, _), _) => false,
            (_, (PathBlock::Unreachable, _)) => false,
            _ => true,
        }
    }

    pub fn place_wall(&mut self, x: usize, y: usize, orientation: WallOrientation) {
        if !self.wall_moves[y][x][match orientation {
            WallOrientation::Horizontal => 0,
            WallOrientation::Vertical => 1,
        }] {
            panic!("wall already removed");
        }

        self.game.board.walls.0[x][y] = Some(orientation);

        self.wall_moves[y][x][0] = false;
        self.wall_moves[y][x][1] = false;
        match orientation {
            WallOrientation::Horizontal => {
                if x > 0 {
                    self.wall_moves[y][x - 1][0] = false;
                }
                if x < WALL_GRID_WIDTH - 1 {
                    self.wall_moves[y][x + 1][0] = false;
                }
            }
            WallOrientation::Vertical => {
                if y > 0 {
                    self.wall_moves[y - 1][x][1] = false;
                }
                if y < WALL_GRID_HEIGHT - 1 {
                    self.wall_moves[y + 1][x][1] = false;
                }
            }
        }
    }
}

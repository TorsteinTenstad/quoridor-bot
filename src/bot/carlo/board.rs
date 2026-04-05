use crate::{
    bot::carlo::bfs::{Bfs, PathBlock},
    data_model::{
        Game, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked,
        is_move_piece_legal_with_players_at_positions, new_position_after_move_piece_unchecked,
    },
};

#[derive(Clone)]
pub struct Board {
    pub game: Game,
    pub bfs_white: Bfs,
    pub bfs_black: Bfs,
    wall_moves: [[[bool; 2]; WALL_GRID_WIDTH]; WALL_GRID_HEIGHT],
}

#[derive(Debug, Default, Clone)]
pub struct BoardStats {
    pub dist: [usize; 2],
    pub walls: [usize; 2],
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        // TODO: one redundant clone
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
        let player = self.game.player;
        self.game = execute_move_unchecked(&self.game, &m);
        match m {
            PlayerMove::MovePiece(_) => {
                let pos = self.game.board.player_position(player);
                if player == Player::White {
                    self.bfs_white.invalidate(&self.game);
                    self.bfs_white.dir = (
                        self.bfs_white.path[pos.y][pos.x].0,
                        self.bfs_white.path[pos.y][pos.x].1 as usize,
                    );
                    if self.bfs_white.dir.0 == PathBlock::Unreachable {
                        self.bfs_white.re_bfs(&self.game);
                    } else {
                        self.bfs_white.recalculate_path(&self.game);
                    }
                }
                if player == Player::Black {
                    self.bfs_black.invalidate(&self.game);
                    self.bfs_black.dir = (
                        self.bfs_black.path[pos.y][pos.x].0,
                        self.bfs_black.path[pos.y][pos.x].1 as usize,
                    );
                    if self.bfs_black.dir.0 == PathBlock::Unreachable {
                        self.bfs_black.re_bfs(&self.game);
                    } else {
                        self.bfs_black.recalculate_path(&self.game);
                    }
                }
            }
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
        }
    }

    pub fn moves(&self) -> impl Iterator<Item = (PlayerMove, Option<BoardStats>)> {
        let player_moves = {
            let p1 = self.game.board.player_position(self.game.player);
            let p2 = self.game.board.player_position(self.game.player.opponent());

            all_move_piece_moves(p1, p2)
                .filter(|m| {
                    is_move_piece_legal_with_players_at_positions(&self.game.board.walls, p1, p2, m)
                })
                .map(|m| {
                    let pos = new_position_after_move_piece_unchecked(p1, &m, p2);

                    let white_dist = match self.game.player {
                        Player::White => {
                            if self.bfs_white.path[pos.y][pos.x].0 != PathBlock::Unreachable {
                                self.bfs_white.path[pos.y][pos.x].1 as usize
                            } else {
                                // BFS terminates when reaching player - must continue.
                                let mut b = self.clone();
                                b.play_move(PlayerMove::MovePiece(m.clone()));
                                b.bfs_white.dir.1 as usize
                            }
                        }
                        Player::Black => self.bfs_white.dir.1,
                    };
                    let black_dist = match self.game.player {
                        Player::Black => {
                            if self.bfs_black.path[pos.y][pos.x].0 != PathBlock::Unreachable {
                                self.bfs_black.path[pos.y][pos.x].1 as usize
                            } else {
                                // BFS terminates when reaching player - must continue.
                                let mut b = self.clone();
                                b.play_move(PlayerMove::MovePiece(m.clone()));
                                b.bfs_black.dir.1 as usize
                            }
                        }
                        Player::White => self.bfs_black.dir.1,
                    };
                    (
                        PlayerMove::MovePiece(m),
                        Some(BoardStats {
                            dist: [white_dist, black_dist],
                            walls: self.game.walls_left,
                        }),
                    )
                })
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

                            let (valid, stats) = self.valid_wall(x, y, orientation);
                            if !valid {
                                return None;
                            }

                            Some((
                                PlayerMove::PlaceWall {
                                    orientation,
                                    position: WallPosition { x, y },
                                },
                                stats,
                            ))
                        })
                    })
                });

        player_moves.chain(wall_moves)
    }

    pub fn valid_wall(
        &self,
        x: usize,
        y: usize,
        orientation: WallOrientation,
    ) -> (bool, Option<BoardStats>) {
        let mut board = self.clone();
        board.place_wall(x, y, orientation);
        board.game.walls_left[self.game.player.as_index()] -= 1;

        board
            .bfs_white
            .recalculate_bfs(&board.game, x, y, orientation);
        if board.bfs_white.dir.0 == PathBlock::Unreachable {
            return (false, None);
        }

        board
            .bfs_black
            .recalculate_bfs(&board.game, x, y, orientation);
        if board.bfs_black.dir.0 == PathBlock::Unreachable {
            return (false, None);
        }

        (
            true,
            Some(BoardStats {
                dist: [board.bfs_white.dir.1, board.bfs_black.dir.1],
                walls: board.game.walls_left,
            }),
        )
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

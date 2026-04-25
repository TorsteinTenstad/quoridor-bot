use crate::{
    bot::carlo::{
        board::{Board, BoardStats},
        mcts,
    },
    data_model::PlayerMove,
    game_logic::execute_move_unchecked,
};
use rand::{Rng, RngExt};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Node {
    pub id: u64,
    pub stats: Option<BoardStats>,
    pub games: usize,
    pub wins: usize,
    pub finished: bool,
}

impl Node {
    pub fn q(&self) -> f64 {
        if self.games > 0 {
            self.wins as f64 / self.games as f64
        } else {
            0.5
        }
    }

    pub fn u(&self, games: usize) -> f64 {
        if self.games == 0 {
            return 1000_f64;
        }
        1_f64 * f64::sqrt(f64::log2(games.max(1) as f64) / self.games as f64)
    }

    pub fn score(&self, games: usize) -> f64 {
        -self.q() + self.u(games)
    }

    pub fn expand(&mut self, ts: &mut mcts::Mcts, board: &Board) {
        if self.finished {
            panic!("already finished")
        }
        if ts.children.get(&self.id) != None {
            panic!("already expanded")
        }

        let children: Vec<(PlayerMove, u64)> = board
            .non_wait_moves()
            .map(|(m, stats)| {
                let game = execute_move_unchecked(&board.game, &m);

                let hash = ts.add_node(&game, stats);
                (m, hash)
            })
            .collect();

        ts.children.insert(self.id, children);
    }

    pub fn pick_move(
        &mut self,
        ts: &mut mcts::Mcts,
        board: &Board,
        visited: &HashMap<u64, usize>,
        explore: bool,
    ) -> Option<(PlayerMove, u64)> {
        if self.finished {
            panic!("cannot get move from finished game")
        }
        if ts.children.get(&self.id) == None {
            self.expand(ts, board);
        }

        let mut rand = rand::rng();
        ts.children
            .get(&self.id)
            .expect("just expanded")
            .iter()
            .map(|(m, hash)| {
                let child = ts.nodes.get(hash).expect("all child nodes exists in tree");

                // heuristic
                let _ = match child.stats {
                    Some(ref stats) => {
                        let self_i = board.game.player.as_index();
                        let other_i = 1 - self_i;

                        if stats.dist[other_i] == 1 {
                            -1000_f64
                        } else {
                            let delta_dist =
                                (stats.dist[other_i] as isize - stats.dist[self_i] as isize) as f64;
                            let delta_walls = (stats.walls[self_i] as isize
                                - stats.walls[other_i] as isize)
                                as f64;

                            delta_dist + delta_walls * 1.5f64
                        }
                    }
                    None => 0f64,
                };

                let r: f64 = rand.random();
                (
                    if explore {
                        child.score(self.games) + r / 10000_f64
                            - 10_f64 * *visited.get(hash).unwrap_or(&0) as f64
                    } else {
                        -child.q()
                    },
                    m,
                    hash,
                )
            })
            .max_by(|(x, _, _), (y, _, _)| x.total_cmp(y))
            .map(|(_, m, hash)| (m.clone(), hash.clone()))
    }
}

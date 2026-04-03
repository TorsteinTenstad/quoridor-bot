use std::collections::HashMap;

use crate::{
    bot::carlo::{board::Board, mcts},
    data_model::PlayerMove,
    game_logic::execute_move_unchecked,
};

#[derive(Default, Clone, Debug)]
pub struct Node {
    pub id: u64,
    pub self_dist: Option<usize>,
    pub other_dist: Option<usize>,
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
            .moves()
            .map(|(m, self_dist, other_dist)| {
                let game = execute_move_unchecked(&board.game, &m);

                let hash = ts.add_node(&game, Some(self_dist), Some(other_dist));
                (m, hash)
            })
            .collect();

        if children.len() == 0 {
            println!("{:?}", board.game);
            println!("{:?}", board.bfs_black);
            println!("{:?}", board.bfs_white);
        }

        ts.children.insert(self.id, children);
    }

    pub fn pick_move(
        &mut self,
        ts: &mut mcts::Mcts,
        board: &Board,
        visited: &HashMap<u64, usize>,
        explore: bool,
    ) -> (PlayerMove, u64) {
        if self.finished {
            panic!("cannot get move from finished game")
        }
        if ts.children.get(&self.id) == None {
            self.expand(ts, board);
        }

        ts.children
            .get(&self.id)
            .expect("just expanded")
            .iter()
            .map(|(m, hash)| {
                let child = ts.nodes.get(hash).expect("all child nodes exists in tree");
                (
                    if explore {
                        child.score(self.games)
                            - 1000f64 * (visited.get(hash).cloned().unwrap_or(0) as f64)
                            - (child.self_dist.unwrap_or(0) as f64) / 20f64
                        // + (child.other_dist.unwrap_or(0) as f64) / 40f64
                    } else {
                        -child.q() - (child.self_dist.unwrap_or(0) as f64) / 20f64
                        // + (child.other_dist.unwrap_or(0) as f64) / 40f64
                    },
                    m,
                    hash,
                )
            })
            .max_by(|(x, _, _), (y, _, _)| x.total_cmp(y))
            .map(|(_, m, hash)| (m.clone(), hash.clone()))
            .expect("all non-final nodes have children")
    }
}

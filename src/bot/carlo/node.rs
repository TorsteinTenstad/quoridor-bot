use std::collections::HashMap;

use crate::{
    bot::carlo::{board::Board, mcts},
    data_model::PlayerMove,
    game_logic::execute_move_unchecked,
};

#[derive(Default, Clone)]
pub struct Node {
    pub id: u64,
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
        1_f64 * f64::sqrt(f64::log(games.max(1) as f64, 2 as f64) / (1 + self.games) as f64)
    }

    pub fn score(&self, games: usize) -> f64 {
        -self.q() + self.u(games)
    }

    pub fn expand(&mut self, ts: &mut mcts::Mcts, board: &Board) {
        if self.finished {
            panic!("already finished")
        }
        if ts.children.get(&self.id) != None || self.finished {
            panic!("already expanded")
        }

        let children = board
            .moves()
            .map(|m| {
                let game = execute_move_unchecked(&board.game, &m);

                let hash = ts.add_node(&game);
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
    ) -> (PlayerMove, u64) {
        if self.finished {
            panic!("cannot get move from finished game")
        }
        if ts.children.get(&self.id) == None {
            self.expand(ts, board);
        }

        // if ts
        //     .children
        //     .get(&self.id)
        //     .expect("just expanded")
        //     .iter()
        //     .filter(|(m, hash)| !visited.contains(hash))
        //     .count()
        //     == 0
        // {
        //     println!("pp{:?}", board.game.board.player_positions)
        // }

        ts.children
            .get(&self.id)
            .expect("just expanded")
            .iter()
            .map(|(m, hash)| {
                let child = ts.nodes.get(hash).expect("all child nodes exists in tree");
                (
                    if explore {
                        -child.q()
                    } else {
                        child.score(self.games)
                    } - (visited.get(hash).cloned().unwrap_or(0) as f64),
                    m,
                    hash,
                )
            })
            .max_by(|(x, _, _), (y, _, _)| x.total_cmp(y))
            .map(|(_, m, hash)| (m.clone(), hash.clone()))
            .expect("all non-final nodes have children")
    }
}

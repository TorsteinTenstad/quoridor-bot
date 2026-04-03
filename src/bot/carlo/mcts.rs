use crate::{
    args::DEFAULT_DURATION,
    bot::carlo::{bfs::game_winner, board::Board},
    data_model::{Game, PlayerMove},
};

use super::node::Node;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

#[derive(Default)]
pub struct Mcts {
    pub nodes: HashMap<u64, Node>,
    pub children: HashMap<u64, Vec<(PlayerMove, u64)>>,
}

impl Mcts {
    pub fn add_node(
        &mut self,
        game: &Game,
        self_dist: Option<usize>,
        other_dist: Option<usize>,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        game.hash(&mut hasher);
        let hash = hasher.finish();

        if self.nodes.contains_key(&hash) {
            return hash;
        }

        let mut node = Node::default();
        node.finished = game_winner(&game) != None;
        node.self_dist = self_dist;
        node.other_dist = other_dist;
        node.id = hash;

        self.nodes.insert(hash, node);
        hash
    }

    fn get_node_by_state(&mut self, game: &Game) -> &mut Node {
        let hash = self.add_node(game, None, None);
        self.nodes.get_mut(&hash).expect("just added")
    }

    pub fn get_move(&mut self, root_game: &Game) -> PlayerMove {
        let mut root = self.get_node_by_state(root_game).clone();
        let root_board = Board::from(root_game);

        let mut sims = 0;
        let mut depth = 0;

        let start_time = std::time::Instant::now();
        while start_time.elapsed() < DEFAULT_DURATION {
            sims += 1;
            let mut node = self.nodes.get(&root.id).unwrap().to_owned();
            let mut stack = vec![root.id];
            let mut board = root_board.clone();
            let mut visited = HashMap::<u64, usize>::new();
            let mut finished = node.finished;
            while !finished {
                // println!("{:?}", board.game.board.player_positions[0]);
                // println!("{:?}", board.bfs_white);
                // println!("{:?}", board.bfs_black);
                // println!(
                //     "{} {} {:?}",
                //     board.bfs_white.queue_i, board.bfs_white.queue_end, board.bfs_white.queue
                // );
                depth += 1;
                visited.insert(node.id, visited.get(&node.id).cloned().unwrap_or(0) + 1);
                let (m, child) = node.pick_move(self, &board, &visited, true);
                stack.push(child);
                node = self.nodes.get(&child).unwrap().clone();
                finished = node.finished;

                board.play_move(m);

                if stack.len() > 128 {
                    break;
                    println!("{:?}", board.game);
                    println!("{:?}", board.bfs_white.dir);
                    println!("{:?}", board.bfs_white);
                    println!("{:?}", board.bfs_black.dir);
                    println!("{:?}", board.bfs_black);
                    println!();
                    println!("{}: {:?}", "r", self.nodes.get(&node.id).unwrap());
                    for (m, c) in self.children.get(&node.id).unwrap() {
                        let c = self.nodes.get(c).unwrap();
                        println!(
                            "{}: {}/{} {} d:{:?}-{:?}",
                            "c", c.wins, c.games, m, c.self_dist, c.other_dist
                        );
                    }
                    panic!();
                }
            }

            let winner = game_winner(&board.game);
            if winner == None {
                for n in stack.into_iter() {
                    let no = self.nodes.get_mut(&n).unwrap();
                    no.wins += 1;
                    no.games += 2;
                }
            } else {
                let mut win = winner == Some(root_game.player);
                for n in stack.into_iter() {
                    let no = self.nodes.get_mut(&n).unwrap();
                    if win {
                        no.wins += 1;
                    }
                    no.games += 1;

                    win = !win;
                }
            }
        }

        println!("{:?}", root_board.bfs_white);
        println!("{:?}", root_board.bfs_black);
        println!("{}: {:?}", "r", self.nodes.get(&root.id).unwrap());
        for (m, c) in self.children.get(&root.id).unwrap() {
            let c = self.nodes.get(c).unwrap();
            println!(
                "{}: {}/{} {} d:{:?}-{:?}",
                "c", c.wins, c.games, m, c.self_dist, c.other_dist
            );
        }
        println!("sims: {}, avg. depth: {}", sims, depth / sims);

        let board = Board::from(root_game);
        root.pick_move(self, &board, &HashMap::<u64, usize>::new(), false)
            .0
    }
}

use crate::{
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
    pub fn add_node(&mut self, game: &Game) -> u64 {
        let mut hasher = DefaultHasher::new();
        game.hash(&mut hasher);
        let hash = hasher.finish();

        if self.nodes.contains_key(&hash) {
            return hash;
        }

        let mut node = Node::default();
        node.finished = game_winner(&game) != None;
        node.id = hash;

        self.nodes.insert(hash, node);
        hash
    }

    fn get_node_by_state(&mut self, game: &Game) -> &mut Node {
        let hash = self.add_node(game);
        self.nodes.get_mut(&hash).expect("just added")
    }

    pub fn get_move(&mut self, root_game: &Game) -> PlayerMove {
        let mut root = self.get_node_by_state(root_game).clone();
        let root_board = Board::from(root_game);

        let start_time = std::time::Instant::now();
        while start_time.elapsed() < Duration::from_secs(5) {
            let mut node = self.nodes.get(&root.id).unwrap().to_owned();
            let mut stack = vec![root.id];
            let mut board = root_board.clone();
            let mut visited = HashMap::<u64, usize>::new();
            let mut finished = node.finished;
            while !finished {
                visited.insert(node.id, visited.get(&node.id).cloned().unwrap_or(0) + 1);
                let (m, child) = node.pick_move(self, &board, &visited, true);
                stack.push(child);
                node = self.nodes.get(&child).unwrap().clone();
                finished = node.finished;

                board.play_move(m);
            }

            let mut win = game_winner(&board.game) == Some(root_game.player);
            for n in stack.into_iter() {
                let no = self.nodes.get_mut(&n).unwrap();
                if win {
                    no.wins += 1;
                }
                no.games += 1;

                win = !win;
            }
        }

        println!("{}: {:?}", "r", self.nodes.get(&root.id).unwrap());
        for (_, c) in self.children.get(&root.id).unwrap() {
            println!("{}: {:?}", "c", self.nodes.get(c).unwrap());
        }

        let board = Board::from(root_game);
        root.pick_move(self, &board, &HashMap::<u64, usize>::new(), false)
            .0
    }
}

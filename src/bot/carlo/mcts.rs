use crate::{
    bot::carlo::board::Board,
    data_model::{Game, PIECE_GRID_HEIGHT, Player, PlayerMove},
};

use super::node::Node;
use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
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

        let finished = game.board.player_position(game.player.opponent()).y
            == match game.player.opponent() {
                Player::White => PIECE_GRID_HEIGHT - 1,
                Player::Black => 0,
            };
        let mut node = Node::default();
        node.finished = finished;
        node.id = hash;

        self.nodes.insert(hash, node);
        hash
    }

    fn get_node_by_state(&mut self, game: &Game) -> &mut Node {
        let hash = self.add_node(game);
        self.nodes.get_mut(&hash).expect("just added")
    }

    pub fn get_move(&mut self, game: &Game) -> PlayerMove {
        let mut root = self.get_node_by_state(game).clone();

        for i in 0..10000 {
            if i % 1000 == 0 {
                println!("{}/10000", i);
            }
            let mut node = root.clone();
            let mut board = Board::from(game);
            let mut visited = HashMap::<u64, usize>::new();
            let mut stack = vec![node.id];
            let mut finished = node.finished;
            while !finished {
                visited.insert(node.id, visited.get(&node.id).cloned().unwrap_or(0) + 1);
                // println!("{} {}", stack.len(), node.id);
                let (m, child) = node.pick_move(self, &board, &visited, true);
                // println!("{:?} {} {}", m, node.id, child);
                stack.push(child);
                node = self.nodes.get(&child).unwrap().clone();
                finished = node.finished;

                board.play_move(m);
            }
            let mut win = false;
            for n in stack.into_iter().rev() {
                let no = self.nodes.get_mut(&n).unwrap();
                if win {
                    no.wins += 1;
                }
                no.games += 1;
                win = !win
            }
        }

        let board = Board::from(game);
        root.pick_move(self, &board, &HashMap::<u64, usize>::new(), false)
            .0
    }
}

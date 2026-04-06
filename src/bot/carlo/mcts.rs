use super::node::Node;
use crate::{
    bot::carlo::{
        bfs::game_winner,
        board::{Board, BoardStats},
    },
    data_model::{Game, Player, PlayerMove},
};
use rand::{rng, seq::IteratorRandom};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

#[derive(Default)]
pub struct Mcts {
    pub nodes: HashMap<u64, Node>,
    pub children: HashMap<u64, Vec<(PlayerMove, u64)>>,
    pub default_seconds: u64,
}

impl Mcts {
    pub fn add_node(&mut self, game: &Game, stats: Option<BoardStats>) -> u64 {
        let mut hasher = DefaultHasher::new();
        game.hash(&mut hasher);
        let hash = hasher.finish();

        if self.nodes.contains_key(&hash) {
            return hash;
        }

        let mut node = Node::default();
        node.finished = game_winner(&game) != None;
        node.stats = stats;
        node.id = hash;

        self.nodes.insert(hash, node);
        hash
    }

    fn get_node_by_state(&mut self, game: &Game) -> &mut Node {
        let hash = self.add_node(game, None);
        self.nodes.get_mut(&hash).expect("just added")
    }

    pub fn get_move(&mut self, root_game: &Game) -> PlayerMove {
        let mut root = self.get_node_by_state(root_game).clone();
        let mut root_board = Board::from(root_game);

        // Don't bother calculating past x wall moves.
        root_board.game.walls_left[0] = root_board.game.walls_left[0].min(4);
        root_board.game.walls_left[1] = root_board.game.walls_left[1].min(4);

        let mut sims = 0;
        let mut total_depth = 0;

        let start_time = std::time::Instant::now();
        while start_time.elapsed() < Duration::from_secs(self.default_seconds) {
            sims += 1;
            let mut node = self.nodes.get(&root.id).unwrap().to_owned();
            let mut stack = vec![root.id];
            let mut board = root_board.clone();
            let mut visited = HashMap::<u64, usize>::new();
            let mut finished = node.finished;
            let mut depth = 0;
            while !finished {
                // println!("{:?}", board.game.board.player_positions[0]);
                // println!("{:?}", board.bfs_white);
                // println!("{:?}", board.bfs_black);
                // println!(
                //     "{} {} {:?}",
                //     board.bfs_white.queue_i, board.bfs_white.queue_end, board.bfs_white.queue
                // );
                let store = depth < 80000;
                let mve = if store {
                    node.pick_move(self, &board, &visited, true)
                } else {
                    Some((
                        board
                            .non_wait_moves()
                            .into_iter()
                            .choose(&mut rng())
                            .unwrap()
                            .0,
                        0,
                    ))
                };
                let (m, child) = match mve {
                    Some((m, child)) => (m, child),
                    None => {
                        println!("{:?}", board.game);
                        println!("{:?}", board.bfs_white.printable(&board.game));
                        println!("{:?}", board.bfs_black.printable(&board.game));
                        panic!();
                    }
                };
                if store {
                    stack.push(child);
                    node = self.nodes.get(&child).unwrap().clone();
                    visited.insert(node.id, visited.get(&node.id).unwrap_or(&0) + 1);
                    finished = node.finished;
                }

                board.play_move(m);

                if !store {
                    finished = game_winner(&board.game) != None;
                }

                // if depth > 32 {
                //     break;
                // }

                if depth > 100000 {
                    println!("{:?}", board.game);
                    println!("{:?}", board.bfs_white.dir);
                    println!("{:?}", board.bfs_white);
                    println!("{:?}", board.bfs_black.dir);
                    println!("{:?}", board.bfs_black);
                    println!();
                    println!("{}: {:?}", "r", self.nodes.get(&node.id).unwrap());
                    if store {
                        for (m, c) in self.children.get(&node.id).unwrap() {
                            let c = self.nodes.get(c).unwrap();
                            println!("{}: {}/{} {}", "c", c.wins, c.games, m);
                        }
                    }
                    panic!();
                }
                total_depth += 1;
                depth += 1;
            }

            //let winner = game_winner(&board.game);
            let winner = if board.bfs_white.dir.1 == board.bfs_black.dir.1 {
                None
            } else {
                if board.bfs_white.dir.1 < board.bfs_black.dir.1 {
                    Some(Player::White)
                } else {
                    Some(Player::Black)
                }
            };
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
            println!("{}: {}/{} {} {:?}", "c", c.wins, c.games, m, c.stats);
        }
        println!("sims: {}, avg. depth: {}", sims, total_depth / sims);

        let board = Board::from(root_game);
        root.pick_move(self, &board, &HashMap::<u64, usize>::new(), false)
            .expect("node has children")
            .0
    }
}

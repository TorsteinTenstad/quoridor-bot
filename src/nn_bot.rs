use burn::backend::NdArray;
use burn::backend::Autodiff;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use rand::distributions::WeightedIndex;
use burn;
use burn::nn::{self, Initializer, Relu};
use burn::tensor::{backend::Backend, Tensor, activation};
use burn::module::{Module, AutodiffModule};
use burn::nn::conv::{Conv2d, Conv2dConfig};
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use std::path::Path;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use rand_distr::{Gamma, Distribution};

use crate::all_moves::ALL_MOVES;
use crate::game_logic::{is_move_legal, execute_move_unchecked};

pub type ActionId = u16;

/// Encoded input planes for the NN. Shape: C x 9 x 9 flattened to row-major.
#[derive(Clone)]
pub struct EncodedState {
    pub planes: Vec<Vec<Vec<f32>>>, // length = C*9*9
    pub c: usize,                   // channels
}

/// Mask of legal actions aligned with the fixed action space.
#[derive(Clone)]
pub struct ActionMask(pub [bool; ACTIONS]);

pub const ACTIONS: usize = 178; // Total moves: 16 pawn moves + 162 wall placements (8x8 grid x 2 orientations + some boundary)

pub type PositionKey = u64;

/// Check if the game is over and return the winner
pub fn is_game_over(game: &Game) -> Option<Player> {
    // White wins if reaches y=8
    if game.board.player_position(Player::White).y() == 8 {
        return Some(Player::White);
    }
    // Black wins if reaches y=0
    if game.board.player_position(Player::Black).y() == 0 {
        return Some(Player::Black);
    }
    None
}

/// Get terminal value from a specific player's perspective
/// Returns Some(1.0) if player won, Some(-1.0) if player lost, None if game not over
pub fn terminal_value_for_player(game: &Game, player: Player) -> Option<f32> {
    is_game_over(game).map(|winner| {
        if winner == player { 1.0 } else { -1.0 }
    })
}

/// Get terminal value from current player's perspective
pub fn terminal_value(game: &Game) -> Option<f32> {
    terminal_value_for_player(game, game.player)
}

/// Hash a game state for transposition table
pub fn game_to_key(game: &Game) -> PositionKey {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    game.board.player_positions[0].index.hash(&mut hasher);
    game.board.player_positions[1].index.hash(&mut hasher);
    // Hash walls
    for x in 0..WALL_GRID_WIDTH {
        for y in 0..WALL_GRID_HEIGHT {
            if let Some(orientation) = game.board.walls[x][y] {
                x.hash(&mut hasher);
                y.hash(&mut hasher);
                (orientation as u8).hash(&mut hasher);
            }
        }
    }
    game.walls_left[0].hash(&mut hasher);
    game.walls_left[1].hash(&mut hasher);
    (game.player as u8).hash(&mut hasher);
    hasher.finish()
}

/// Get all legal action IDs for the current player
pub fn legal_action_ids(game: &Game) -> Vec<ActionId> {
    ALL_MOVES.iter()
        .enumerate()
        .filter(|(_, mv)| is_move_legal(game, game.player, mv))
        .map(|(id, _)| id as ActionId)
        .collect()
}

/// Apply an action to a game state (returns new state)
pub fn apply_action(game: &Game, action_id: ActionId) -> Game {
    let mut new_game = game.clone();
    let player_move = action_from_id(action_id);
    execute_move_unchecked(&mut new_game, game.player, &player_move);
    new_game
}

fn action_from_id(action_id: ActionId) -> PlayerMove {
    ALL_MOVES.get(action_id as usize).unwrap().clone()
}

pub fn get_move(game: &Game, network: &QuoridorNet, player: Player, temperature: f32) -> PlayerMove
{
    let mut rng = thread_rng();

    let encoded = vec![encode(game)];
    let prediction = predict_batch(network, &encoded);

    let legal_moves: Vec<(usize, &f32)> = prediction.first().unwrap().policy_logits.iter().enumerate()
        .filter(|(id, _)|{is_move_legal(game, player, &action_from_id(*id as u16))}).collect();

    // Handle edge case of no legal moves (shouldn't happen in valid game)
    if legal_moves.is_empty() {
        panic!("No legal moves available for player {:?}", player);
    }

    // Apply temperature
    let max_logit = legal_moves
        .iter()
        .map(|&(_, l)| *l)
        .fold(f32::NEG_INFINITY, f32::max);
    let exp_logits: Vec<f32> = legal_moves
        .iter()
        .map(|&(_, logit)| {
            let val = ((logit - max_logit) / temperature).exp();
            if val.is_finite() { val } else { 0.0 }
        })
        .collect();

    // Normalize into probabilities
    let sum_exp: f32 = exp_logits.iter().sum();
    
    // Handle edge case where all probabilities are zero or non-finite
    let probs: Vec<f32> = if sum_exp > 0.0 && sum_exp.is_finite() {
        exp_logits.iter().map(|x| x / sum_exp).collect()
    } else {
        // Fallback to uniform distribution
        vec![1.0 / legal_moves.len() as f32; legal_moves.len()]
    };

    // Sample from distribution
    let dist = WeightedIndex::new(&probs)
        .expect("Failed to create weighted distribution from probabilities");
    let choice = dist.sample(&mut rng);

    // Extract the most likely move from the output
    action_from_id(legal_moves[choice].0 as u16)
}

fn encode(game: &Game) -> EncodedState {
    // shape: [channels, 9, 9]
    let mut channels = vec![vec![vec![0.0; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT]; 8];

    // player pawns
    for p in [Player::White, Player::Black] {
        let pos = game.board.player_position(p);
        channels[p.as_index()][pos.y()][pos.x()] = 1.0;
    }

    // walls (just fill in as 1.0 where a wall is placed)
    for x in 0..WALL_GRID_WIDTH {
        for y in 0..WALL_GRID_HEIGHT {
            if let Some(o) = game.board.walls[x][y] {
                match o {
                    WallOrientation::Horizontal => channels[2][y][x] = 1.0,
                    WallOrientation::Vertical => channels[3][y][x] = 1.0,
                }
            }
        }
    }

    // walls left (normalized by 10)
    for x in 0..PIECE_GRID_WIDTH {
        for y in 0..PIECE_GRID_HEIGHT {
            channels[4][y][x] = game.walls_left[0] as f32 / 10.0;
            channels[5][y][x] = game.walls_left[1] as f32 / 10.0;
        }
    }

    // player-to-move plane
    let current = game.player.as_index();
    for x in 0..PIECE_GRID_WIDTH {
        for y in 0..PIECE_GRID_HEIGHT {
            channels[6][y][x] = if current == 0 { 1.0 } else { 0.0 };
        }
    }

    EncodedState {
        planes: channels,
        c: 8,
    }
}

// ===== 1) Policy-Value Network interface =====

/// Output of a network forward pass on a single position.
#[derive(Clone)]
pub struct NetOut {
    pub policy_logits: [f32; ACTIONS], // unnormalized
    pub value: f32,                    // in [-1, 1]
}

/// Backend-agnostic network interface. Implement with `burn`, `tch`, `candle`, etc.
pub trait PolicyValueNet: Send + 'static {
    /// Inference on a *batch* of encoded states. Must be thread-safe; do batching on GPU here.
    fn predict_batch(&self, batch: &[EncodedState]) -> Vec<NetOut>;

    /// Optional training step. Provide your own optimizer + loss inside.
    /// Return (policy_loss, value_loss).
    fn train_step(&mut self, _batch: &[(EncodedState, [f32; ACTIONS], f32)]) -> (f32, f32) {
        (0.0, 0.0)
    }
}

#[derive(Clone, Default)]
struct EdgeStats {
    n: u32,   // visit count
    w: f32,   // total value
    q: f32,   // mean value
    p: f32,   // prior
}

#[derive(Clone, Default)]
struct Node {
    // edges indexed by ActionId; present only for legal actions
    edges: HashMap<ActionId, EdgeStats>,
    // cache terminal or expanded
    expanded: bool,
    // optional: value estimate at node creation
    _v0: f32,
}

#[derive(Clone)]
pub struct MctsConfig {
    pub c_puct: f32,           // ~1.5
    pub dirichlet_alpha: f32,  // ~0.3
    pub dirichlet_eps: f32,    // ~0.25
    pub simulations: usize,    // 200..800
    pub root_noise: bool,
    pub temperature: f32,      // for move selection from visits
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            c_puct: 1.5,
            dirichlet_alpha: 0.3,
            dirichlet_eps: 0.25,
            simulations: 400,
            root_noise: true,
            temperature: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct Mcts {
    cfg: MctsConfig,
    net: QuoridorNet,
    // Transposition table: key -> node
    nodes: HashMap<PositionKey, Node>,
    rng: ThreadRng,
}

impl Mcts {
    pub fn new(cfg: MctsConfig, net: QuoridorNet) -> Self {
        Self { cfg, net, nodes: HashMap::new(), rng: thread_rng() }
    }

    fn get_or_expand(&mut self, state: &Game) -> (PositionKey, bool) {
        let key = game_to_key(state);
        let is_new = !self.nodes.contains_key(&key);
        if is_new {
            // evaluate with net
            let enc = vec![encode(state)];
            let out = predict_batch(&self.net, &enc)[0].clone();
            let legal = legal_action_ids(state);

            // softmax over legal only
            let logits = out.policy_logits;
            let max_logit = logits.iter().cloned().reduce(f32::max).unwrap_or(0.0);
            let mut sum = 0f32;
            let mut p = [0f32; ACTIONS];
            for &a in &legal {
                let z = (logits[a as usize] - max_logit).exp();
                p[a as usize] = z;
                sum += z;
            }
            if sum > 0.0 {
                for &a in &legal { p[a as usize] /= sum; }
            }

            let mut edges = HashMap::with_capacity(legal.len());
            for &a in &legal {
                edges.insert(a, EdgeStats { n: 0, w: 0.0, q: 0.0, p: p[a as usize] });
            }

            self.nodes.insert(key, Node { edges, expanded: true, _v0: out.value });
        }
        (key, is_new)
    }

    pub fn run(&mut self, root: &Game) -> [f32; ACTIONS] {
        // Ensure root exists
        let (root_key, _) = self.get_or_expand(root);

        // Dirichlet noise on root priors for exploration
        if self.cfg.root_noise {
            if let Some(node) = self.nodes.get_mut(&root_key) {
                let k = node.edges.len().max(1);
                let alpha = self.cfg.dirichlet_alpha as f64;
                let mut draws = Vec::with_capacity(k);
                let mut sum = 0.0;
                let gamma_dist = Gamma::new(alpha, 1.0).unwrap();
                for _ in 0..k { 
                    let g: f64 = gamma_dist.sample(&mut self.rng);
                    draws.push(g);
                    sum += g;
                }
                if sum > 0.0 {
                    let mut i = 0usize;
                    for (_a, e) in node.edges.iter_mut() {
                        let noise = draws[i] / sum; i += 1;
                        e.p = (1.0 - self.cfg.dirichlet_eps) * e.p + self.cfg.dirichlet_eps * noise as f32;
                    }
                }
            }
        }

        for _ in 0..self.cfg.simulations {
            let mut path: Vec<(PositionKey, ActionId)> = Vec::with_capacity(64);
            let mut state = root.clone();
            let mut player_sign = 1.0f32; // value is from current player POV
            let mut visited_keys = std::collections::HashSet::new();

            // Selection
            loop {
                let key = game_to_key(&state);
                
                // Check for cycles - if we've seen this position in this simulation, break
                if visited_keys.contains(&key) {
                    // Cycle detected - treat as draw and backup
                    self.backup(&path, 0.0);
                    break;
                }
                visited_keys.insert(key);
                
                if !self.nodes.contains_key(&key) { break; }
                let node = self.nodes.get(&key).unwrap();

                // terminal check before selecting
                if let Some(v) = terminal_value(&state) {
                    // backup terminal directly
                    self.backup(&path, v * player_sign);
                    path.clear();
                    break;
                }

                // choose action maximizing PUCT
                let mut best = None;
                let sum_n: f32 = node.edges.values().map(|e| e.n as f32).sum();
                for (&a, e) in node.edges.iter() {
                    let u = e.q + self.cfg.c_puct * e.p * ((sum_n + 1e-8).sqrt() / (1.0 + e.n as f32));
                    if best.map(|(_aa, bb)| u > bb).unwrap_or(true) {
                        best = Some((a, u));
                    }
                }
                let (a_sel, _score) = best.expect("no legal moves in non-terminal state");
                path.push((key, a_sel));
                state = apply_action(&state, a_sel);
                player_sign = -player_sign;

                // expansion condition: if child not expanded yet
                if !self.nodes.contains_key(&game_to_key(&state)) {
                    // Expand + evaluate leaf
                    let enc = vec![encode(&state)];
                    let out = predict_batch(&self.net, &enc)[0].clone();
                    let legal = legal_action_ids(&state);
                    let logits = out.policy_logits;
                    let max_logit = logits.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                    let mut sum = 0f32;
                    let mut p = [0f32; ACTIONS];
                    for &a in &legal {
                        let z = (logits[a as usize] - max_logit).exp();
                        p[a as usize] = z; sum += z;
                    }
                    if sum > 0.0 { for &a in &legal { p[a as usize] /= sum; } }
                    let mut edges = HashMap::with_capacity(legal.len());
                    for &a in &legal { edges.insert(a, EdgeStats { n: 0, w: 0.0, q: 0.0, p: p[a as usize] }); }
                    self.nodes.insert(game_to_key(&state), Node { edges, expanded: true, _v0: out.value });
                    // backup leaf value (perspective flips already applied via player_sign)
                    self.backup(&path, out.value * player_sign);
                    path.clear();
                    break;
                }
            }
        }

        // Build π from root visit counts
        let node = self.nodes.get(&root_key).unwrap();
        let mut pi = [0f32; ACTIONS];
        for (&a, e) in node.edges.iter() { pi[a as usize] = e.n as f32; }
        // temperature
        if self.cfg.temperature != 1.0 {
            for x in pi.iter_mut() { *x = x.powf(1.0 / self.cfg.temperature.max(1e-6)); }
        }
        let sum: f32 = pi.iter().sum();
        if sum > 0.0 { for x in pi.iter_mut() { *x /= sum; } }
        pi
    }

    fn backup(&mut self, path: &[(PositionKey, ActionId)], mut v: f32) {
        for (key, a) in path.iter().rev() {
            if let Some(node) = self.nodes.get_mut(key) {
                if let Some(e) = node.edges.get_mut(a) {
                    e.n += 1;
                    e.w += v;
                    e.q = e.w / (e.n as f32);
                }
            }
            v = -v; // alternate players
        }
    }
}
//         let key = G::key(s);
//         let is_new = !self.nodes.contains_key(&key);
//         if is_new {
//             // evaluate with net
//             let enc = G::encode(s);
//             let out = self.net.predict_batch(&[enc])[0].clone();
//             let legal = G::legal_actions(s);

//             // softmax over legal only
//             let mut logits = out.policy_logits;
//             let max_logit = logits.iter().cloned().reduce(f32::max).unwrap_or(0.0);
//             let mut sum = 0f32;
//             let mut p = [0f32; ACTIONS];
//             for &a in &legal {
//                 let action_id = G::to_action_id(&a) as usize;
//                 let z = (logits[action_id] - max_logit).exp();
//                 p[action_id] = z;
//                 sum += z;
//             }
//             if sum > 0.0 {
//                 for &a in &legal { p[G::to_action_id(&a) as usize] /= sum; }
//             }

//             let mut edges = HashMap::with_capacity(legal.len());
//             for &a in &legal {
//                 edges.insert(a, EdgeStats { n: 0, w: 0.0, q: 0.0, p: p[G::to_action_id(&a) as usize] });
//             }

//             self.nodes.insert(key, Node::<G> { edges, expanded: true, mask, _v0: out.value, _phantom: Default::default() });
//         }
//         (key, is_new)
//     }

//     pub fn run(&mut self, root: &G::State) -> [f32; ACTIONS] {
//         // Ensure root exists
//         let (root_key, _) = self.get_or_expand(root);

//         // Dirichlet noise on root priors for exploration
//         if self.cfg.root_noise {
//             if let Some(node) = self.nodes.get_mut(&root_key) {
//                 let k = node.edges.len().max(1);
//                 // crude gamma sampling for Dirichlet(alpha)
//                 let alpha = self.cfg.dirichlet_alpha;
//                 let mut draws = Vec::with_capacity(k);
//                 let mut sum = 0.0;
//                 for _ in 0..k { let g = gamma_sample(alpha, &mut self.rng); draws.push(g); sum += g; }
//                 if sum > 0.0 {
//                     let mut i = 0usize;
//                     for (_a, e) in node.edges.iter_mut() {
//                         let noise = draws[i] / sum; i += 1;
//                         e.p = (1.0 - self.cfg.dirichlet_eps) * e.p + self.cfg.dirichlet_eps * noise as f32;
//                     }
//                 }
//             }
//         }

//         for _ in 0..self.cfg.simulations {
//             let mut path: Vec<(PositionKey, ActionId)> = Vec::with_capacity(64);
//             let mut state = root.clone();
//             let mut player_sign = 1.0f32; // value is from current player POV

//             // Selection
//             loop {
//                 let key = G::key(&state);
//                 if !self.nodes.contains_key(&key) { break; }
//                 let node = self.nodes.get(&key).unwrap();

//                 // terminal check before selecting
//                 if let Some(v) = G::terminal_value(&state) {
//                     // backup terminal directly
//                     self.backup(&path, v * player_sign);
//                     path.clear();
//                     break;
//                 }

//                 // choose action maximizing PUCT
//                 let mut best = None;
//                 let sum_n: f32 = node.edges.values().map(|e| e.n as f32).sum();
//                 for (&a, e) in node.edges.iter() {
//                     // mask is redundant here because edges exist only for legal moves
//                     let u = e.q + self.cfg.c_puct * e.p * ((sum_n + 1e-8).sqrt() / (1.0 + e.n as f32));
//                     if best.map(|(_aa, bb)| u > bb).unwrap_or(true) {
//                         best = Some((a, u));
//                     }
//                 }
//                 let (a_sel, _score) = best.expect("no legal moves in non-terminal state");
//                 path.push((key, a_sel));
//                 state = G::apply(&state, a_sel);
//                 player_sign = -player_sign;

//                 // expansion condition: if child not expanded yet
//                 if !self.nodes.contains_key(&G::key(&state)) {
//                     // Expand + evaluate leaf
//                     let enc = G::encode(&state);
//                     let out = self.net.predict_batch(&[enc])[0].clone();
//                     let (legal, mask) = G::legal_actions(&state);
//                     let mut logits = out.policy_logits;
//                     let max_logit = logits.iter().cloned().reduce(f32::max).unwrap_or(0.0);
//                     let mut sum = 0f32;
//                     let mut p = [0f32; ACTIONS];
//                     for &a in &legal {
//                         let z = (logits[a as usize] - max_logit).exp();
//                         p[a as usize] = z; sum += z;
//                     }
//                     if sum > 0.0 { for &a in &legal { p[a as usize] /= sum; } }
//                     let mut edges = HashMap::with_capacity(legal.len());
//                     for &a in &legal { edges.insert(a, EdgeStats { n: 0, w: 0.0, q: 0.0, p: p[a as usize] }); }
//                     self.nodes.insert(G::key(&state), Node::<G> { edges, expanded: true, mask, _v0: out.value, _phantom: Default::default() });
//                     // backup leaf value (perspective flips already applied via player_sign)
//                     self.backup(&path, out.value * player_sign);
//                     path.clear();
//                     break;
//                 }
//             }
//         }

//         // Build π from root visit counts
//         let node = self.nodes.get(&root_key).unwrap();
//         let mut pi = [0f32; ACTIONS];
//         for (&a, e) in node.edges.iter() { pi[a as usize] = e.n as f32; }
//         // temperature
//         if self.cfg.temperature != 1.0 {
//             for x in pi.iter_mut() { *x = x.powf(1.0 / self.cfg.temperature.max(1e-6)); }
//         }
//         let sum: f32 = pi.iter().sum();
//         if sum > 0.0 { for x in pi.iter_mut() { *x /= sum; } }
//         pi
//     }

//     fn backup(&mut self, path: &[(PositionKey, ActionId)], mut v: f32) {
//         for (key, a) in path.iter().rev() {
//             if let Some(node) = self.nodes.get_mut(key) {
//                 if let Some(e) = node.edges.get_mut(a) {
//                     e.n += 1;
//                     e.w += v;
//                     e.q = e.w / (e.n as f32);
//                 }
//             }
//             v = -v; // alternate players
//         }
//     }
// }

// // gamma(alpha, 1) sampler (very rough; replace with statrs or rand_distr if you prefer)
// fn gamma_sample(alpha: f32, rng: &mut ThreadRng) -> f64 {
//     use rand::distributions::{Distribution, Open01};
//     // Marsaglia-Tsang for alpha > 1; for simplicity bump alpha
//     let a = (alpha.max(1.0001) - 1.0) as f64;
//     let d = a; let c = (1.0 / (9.0 * d)).sqrt();
//     loop {
//         let mut x: f64; let mut v: f64;
//         loop {
//             let z: f64 = rand_distr::StandardNormal.sample(rng);
//             x = 1.0 + c * z; if x > 0.0 { v = x * x * x; break; }
//         }
//         let u: f64 = Open01.sample(rng);
//         if u < 1.0 - 0.331 * (z2(v)) { return d * v; }
//         if (u.ln()) < 0.5 * zsq_from_v(v) + d * (1.0 - v + v.ln()) { return d * v; }
//     }
//     fn z2(v: f64) -> f64 { let z = (v.powf(1.0/3.0) - 1.0) / 1.0; z * z }
//     fn zsq_from_v(_v: f64) -> f64 { 0.0 }
// }

// ===== Self-play worker =====

#[derive(Clone)]
pub struct SelfPlayCfg {
    pub sims_per_move: usize,
    pub temperature_moves: usize, // play with τ=1 up to this ply, then τ=0.1
}

impl Default for SelfPlayCfg {
    fn default() -> Self {
        Self {
            sims_per_move: 400,
            temperature_moves: 10,
        }
    }
}

pub struct Trajectory {
    pub encodings: Vec<EncodedState>,
    pub policies: Vec<[f32; ACTIONS]>, // π from visits
    pub players: Vec<i8>,              // +1 or -1, whose POV each state was recorded from
    pub result: f32,                   // final z in [-1,1] from first player's POV
}

pub fn play_one_game(mcts: &mut Mcts, initial_state: Game, sp: &SelfPlayCfg) -> Trajectory {
    let mut encodings = Vec::new();
    let mut policies = Vec::new();
    let mut players = Vec::new();

    let mut ply = 0usize;
    let mut current_state = initial_state;
    let first_player = current_state.player;

    loop {
        // Check for terminal
        if let Some(winner) = is_game_over(&current_state) {
            // Result from first player's POV
            let result = if winner == first_player { 1.0 } else { -1.0 };
            return Trajectory { encodings, policies, players, result };
        }

        // Update MCTS config for this move
        let mut mcts_cfg = mcts.cfg.clone();
        mcts_cfg.simulations = sp.sims_per_move;
        mcts_cfg.temperature = if ply < sp.temperature_moves { 1.0 } else { 0.1 };
        mcts.cfg = mcts_cfg;

        let pi = mcts.run(&current_state);

        // Sample action according to π
        let mut a = sample_from_pi(&pi, &mut thread_rng());
        
        // Verify the selected action is legal
        let legal_moves = legal_action_ids(&current_state);
        if legal_moves.is_empty() {
            eprintln!("ERROR: No legal moves available at ply {}", ply);
            eprintln!("Game state: player={:?}, positions={:?}", 
                     current_state.player, current_state.board.player_positions);
            return Trajectory { encodings, policies, players, result: 0.0 };
        }
        if !legal_moves.contains(&a) {
            eprintln!("ERROR: MCTS selected illegal action {} at ply {}", a, ply);
            eprintln!("Legal moves: {:?}", legal_moves);
            eprintln!("Policy sum: {}, non-zero count: {}", 
                     pi.iter().sum::<f32>(), pi.iter().filter(|&&x| x > 0.0).count());
            // Fallback: pick first legal move
            a = legal_moves[0];
        }

        // Record from current player's perspective
        let player_sign = if current_state.player == first_player { 1 } else { -1 };
        encodings.push(encode(&current_state));
        policies.push(pi);
        players.push(player_sign);

        // Advance
        current_state = apply_action(&current_state, a);
        ply += 1;

        if ply > 200 {
            // Draw - return 0
            return Trajectory { encodings, policies, players, result: 0.0 };
        }
    }
}

fn sample_from_pi(pi: &[f32; ACTIONS], rng: &mut ThreadRng) -> ActionId {
    let sum: f32 = pi.iter().sum();
    if sum <= 0.0 {
        // fallback: pick argmax
        return pi.iter().enumerate()
            .max_by(|a,b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i,_)| i as ActionId)
            .unwrap_or(0);
    }
    
    use rand::Rng as _;
    let r_val: f32 = rng.r#gen();
    let r = r_val * sum;
    let mut acc = 0.0;
    for (i, p) in pi.iter().enumerate() {
        acc += *p;
        if r <= acc { return i as ActionId; }
    }
    (ACTIONS - 1) as ActionId
}

// ===== Replay buffer =====

use std::collections::VecDeque;

pub struct ReplayBuffer {
    buf: VecDeque<(EncodedState, [f32; ACTIONS], f32)>,
    cap: usize,
}

impl ReplayBuffer {
    pub fn new(cap: usize) -> Self { 
        Self { buf: VecDeque::with_capacity(cap), cap } 
    }
    
    pub fn push_game(&mut self, traj: &Trajectory) {
        // Convert each sample to (state, π, z from that state's player POV)
        for i in 0..traj.encodings.len() {
            let player_sign = traj.players[i] as f32;
            // Result is from first player POV, adjust to current state's POV
            let z = traj.result * player_sign;
            self.push(traj.encodings[i].clone(), traj.policies[i], z);
        }
    }
    
    fn push(&mut self, s: EncodedState, pi: [f32; ACTIONS], z: f32) {
        if self.buf.len() == self.cap { 
            self.buf.pop_front(); 
        }
        self.buf.push_back((s, pi, z));
    }
    
    pub fn sample_batch(&self, bs: usize, rng: &mut ThreadRng) -> Vec<(EncodedState, [f32; ACTIONS], f32)> {
        let n = self.buf.len();
        let mut out = Vec::with_capacity(bs);
        for _ in 0..bs { 
            let i = rng.gen_range(0..n);
            out.push(self.buf[i].clone()); 
        }
        out
    }
    
    pub fn len(&self) -> usize { 
        self.buf.len() 
    }
    
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

// ===== Training configuration and loop =====

/// Helper function to create a new NetworkModel on any backend
fn create_network_model<B: Backend>(device: &B::Device) -> NetworkModel<B> {
    let conv_cfg = Conv2dConfig::new([8, 64], [3, 3])
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
    let conv1 = conv_cfg.init(device);

    let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
    let conv2 = conv_cfg2.init(device);

    let fc_policy = nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
        .init(device);

    let fc_value1 = nn::LinearConfig::new(64 * 5 * 5, 64)
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
        .init(device);

    let fc_value2 = nn::LinearConfig::new(64, 1)
        .with_initializer(Initializer::XavierNormal { gain: 1.0 })
        .init(device);

    NetworkModel { conv1, conv2, fc_policy, fc_value1, fc_value2 }
}

#[derive(Clone)]
pub struct TrainCfg {
    pub batch_size: usize,
    pub steps_per_iter: usize,
    pub games_per_iter: usize,
    pub replay_size: usize,
    pub iterations: usize,
}

impl Default for TrainCfg {
    fn default() -> Self {
        Self {
            batch_size: 128,
            steps_per_iter: 100,
            games_per_iter: 10,
            replay_size: 10_000,
            iterations: 100,
        }
    }
}

pub fn train_loop(
    initial_net: &QuoridorNet,
    mcts_cfg: MctsConfig,
    sp_cfg: SelfPlayCfg,
    tcfg: TrainCfg,
    save_path: Option<&std::path::Path>,
) {
    let mut rng = thread_rng();
    let mut replay = ReplayBuffer::new(tcfg.replay_size);
    
    // Create network on autodiff backend for training
    type ADBackend = Autodiff<NdArray>;
    let device = <ADBackend as Backend>::Device::default();
    
    // NOTE: Currently creates fresh random weights on autodiff backend
    // The initial_net weights are not transferred due to record type mismatch
    // This means training always starts from scratch (cannot resume from checkpoint)
    // TODO: Implement proper weight transfer using record conversion or manual param copy
    let mut train_model = create_network_model::<ADBackend>(&device);
    
    let mut optim = AdamConfig::new().init();

    println!("Starting training loop...");
    println!("Config: {} iterations, {} games/iter, {} steps/iter", 
             tcfg.iterations, tcfg.games_per_iter, tcfg.steps_per_iter);
    println!("Learning rate: 0.001 (Adam default)\n");

    for iter in 0..tcfg.iterations {
        println!("\n=== Iteration {}/{} ===", iter + 1, tcfg.iterations);
        
        // Convert training model back to inference model for self-play
        let inference_model: NetworkModel<NdArray> = train_model.clone().valid();
        let current_net = QuoridorNet {
            device: initial_net.device.clone(),
            network_model: inference_model,
        };
        
        // 1) Self-play with current network
        println!("Generating {} self-play games...", tcfg.games_per_iter);
        let mcts_for_play = Mcts::new(mcts_cfg.clone(), current_net);
        
        for game_idx in 0..tcfg.games_per_iter {
            let mut game_mcts = mcts_for_play.clone();
            let initial_state = Game::new();
            let traj = play_one_game(&mut game_mcts, initial_state, &sp_cfg);
            
            let winner = if traj.result > 0.0 { "White" } else if traj.result < 0.0 { "Black" } else { "Draw" };
            println!("  Game {}: {} moves, winner: {}", 
                     game_idx + 1, traj.encodings.len(), winner);
            
            replay.push_game(&traj);
        }
        
        println!("Replay buffer size: {}", replay.len());
        
        // 2) Training phase
        if replay.len() >= tcfg.batch_size {
            println!("Training for {} steps...", tcfg.steps_per_iter);
            let mut total_policy_loss = 0.0;
            let mut total_value_loss = 0.0;
            
            for step in 0..tcfg.steps_per_iter {
                let batch = replay.sample_batch(tcfg.batch_size, &mut rng);
                
                // Perform training step
                let (policy_loss, value_loss) = train_step(&mut train_model, &mut optim, &batch, &device);
                
                total_policy_loss += policy_loss;
                total_value_loss += value_loss;
                
                if step % 20 == 0 && step > 0 {
                    let avg_p = total_policy_loss / (step as f32);
                    let avg_v = total_value_loss / (step as f32);
                    println!("  Step {}/{} - Policy Loss: {:.4}, Value Loss: {:.4}", 
                             step, tcfg.steps_per_iter, avg_p, avg_v);
                }
            }
            
            let avg_policy = total_policy_loss / (tcfg.steps_per_iter as f32);
            let avg_value = total_value_loss / (tcfg.steps_per_iter as f32);
            println!("  Average losses - Policy: {:.4}, Value: {:.4}", avg_policy, avg_value);
        } else {
            println!("Not enough samples for training yet (need {})", tcfg.batch_size);
        }
        
        // 3) Save checkpoint
        if let Some(path) = save_path {
            if (iter + 1) % 10 == 0 || iter + 1 == tcfg.iterations {
                let checkpoint_path = if iter + 1 == tcfg.iterations {
                    path.to_path_buf()
                } else {
                    path.with_file_name(
                        format!("checkpoint_iter_{}.mpk", iter + 1)
                    )
                };
                println!("Saving checkpoint to {:?}", checkpoint_path);
                
                // Convert to inference backend for saving
                let save_model: NetworkModel<NdArray> = train_model.clone().valid();
                let save_net = QuoridorNet {
                    device: initial_net.device.clone(),
                    network_model: save_model,
                };
                save_net.save(&checkpoint_path).expect("Failed to save checkpoint");
            }
        }
    }
    
    // Final save
    if let Some(path) = save_path {
        println!("\nSaving final model to {:?}", path);
        let final_model: NetworkModel<NdArray> = train_model.valid();
        let final_net = QuoridorNet {
            device: initial_net.device.clone(),
            network_model: final_model,
        };
        final_net.save(path).expect("Failed to save final model");
    }
    
    println!("\n✅ Training complete!");
}

/// Perform a single training step with backpropagation
fn train_step<B: burn::tensor::backend::AutodiffBackend>(
    net: &mut NetworkModel<B>,
    optim: &mut burn::optim::adaptor::OptimizerAdaptor<burn::optim::Adam, NetworkModel<B>, B>,
    batch: &[(EncodedState, [f32; ACTIONS], f32)],
    device: &B::Device,
) -> (f32, f32) {
    
    // Encode batch inputs - create tensors directly on the correct backend
    let batch_size = batch.len();
    let c = 8; // channels
    
    // Flatten all inputs into single Vec
    let mut flat: Vec<f32> = Vec::with_capacity(batch_size * c * 9 * 9);
    for (state, _, _) in batch {
        for chan in 0..c {
            for row in 0..9 {
                flat.extend_from_slice(&state.planes[chan][row]);
            }
        }
    }
    
    let input_tensor = Tensor::<B, 4>::from_data(
        burn::tensor::TensorData::new(flat, [batch_size, c, 9, 9]),
        device,
    );
    
    // Target policies and values
    let target_policies: Vec<[f32; ACTIONS]> = batch.iter().map(|(_, p, _)| *p).collect();
    let target_values: Vec<f32> = batch.iter().map(|(_, _, v)| *v).collect();
    
    // Convert targets to tensors on the correct backend
    let policy_target_flat: Vec<f32> = target_policies.iter().flat_map(|p| p.iter().copied()).collect();
    let policy_target = Tensor::<B, 2>::from_data(
        burn::tensor::TensorData::new(policy_target_flat, [batch_size, ACTIONS]),
        device,
    );
    
    let value_target = Tensor::<B, 2>::from_data(
        burn::tensor::TensorData::new(target_values, [batch_size, 1]),
        device,
    );
    
    // Forward pass
    let output = net.forward(input_tensor);
    
    // Compute losses
    // Policy loss: cross-entropy
    let policy_log_softmax = activation::log_softmax(output.policy.clone(), 1);
    let policy_loss = -(policy_target * policy_log_softmax).mean();
    
    // Value loss: Mean Squared Error  
    let value_diff = output.value - value_target;
    let value_loss = (value_diff.clone() * value_diff).mean();
    
    // Combined loss
    let total_loss = policy_loss.clone() + value_loss.clone();
    
    // Backward pass
    let grads = total_loss.backward();
    
    // Update weights
    let grads = GradientsParams::from_grads(grads, net);
    let updated_net = optim.step(1.0, net.clone(), grads);
    *net = updated_net;
    
    // Return loss values for logging - convert to inner backend first
    let policy_inner = policy_loss.clone().inner();
    let value_inner = value_loss.clone().inner();
    let policy_loss_val: f32 = policy_inner.into_data().to_vec().unwrap()[0];
    let value_loss_val: f32 = value_inner.into_data().to_vec().unwrap()[0];
    
    (policy_loss_val, value_loss_val)
}


/// Burn network

// #[derive(Clone)]
// pub struct SelfPlayCfg {
//     pub sims_per_move: usize,
//     pub temperature_moves: usize, // play with τ=1 up to this ply, then τ=0.1
// }

// pub struct Trajectory {
//     pub encodings: Vec<EncodedState>,
//     pub policies: Vec<[f32; ACTIONS]>, // π from visits
//     pub players: Vec<i8>,              // +1 or -1, whose POV each state was recorded from
//     pub result: f32,                   // final z in [-1,1] from player who moved first
// }

// pub fn play_one_game<G: GameAdapter>(mcts: &mut Mcts<G>, mut state: G::State, sp: &SelfPlayCfg) -> Trajectory {
//     let mut encodings = Vec::new();
//     let mut policies = Vec::new();
//     let mut players = Vec::new();

//     let mut ply = 0usize;
//     let mut current_state = state.clone();
//     let mut current_player: i8 = 1; // +1 starts

//     loop {
//         if let Some(v) = G::terminal_value(&current_state) {
//             // assign result from first player's POV
//             let result = v; // Assuming v is from current player's POV; convert to first player's POV:
//             // We stored players, so adjust per sample later.
//             return Trajectory { encodings, policies, players, result };
//         }

//         let mut mcts_cfg = mcts.cfg.clone();
//         mcts_cfg.simulations = sp.sims_per_move;
//         mcts_cfg.temperature = if ply < sp.temperature_moves { 1.0 } else { 0.1 };
//         mcts.cfg = mcts_cfg; // update

//         let pi = mcts.run(&current_state);

//         // sample action according to π (with temperature already applied)
//         let a = sample_from_pi(&pi, &mut rand::thread_rng());

//         // record
//         encodings.push(G::encode(&current_state));
//         policies.push(pi);
//         players.push(current_player);

//         // advance
//         current_state = G::apply(&current_state, a);
//         current_player = -current_player;
//         ply += 1;
//     }
// }

// fn sample_from_pi(pi: &[f32; ACTIONS], rng: &mut ThreadRng) -> ActionId {
//     let mut r: f32 = rng.gen();
//     let sum: f32 = pi.iter().sum();
//     if sum <= 0.0 {
//         // fallback: pick argmax
//         return pi.iter().enumerate().max_by(|a,b| a.1.partial_cmp(b.1).unwrap()).map(|(i,_)| i as ActionId).unwrap_or(0);
//     }
//     r *= sum;
//     let mut acc = 0.0;
//     for (i, p) in pi.iter().enumerate() {
//         acc += *p;
//         if r <= acc { return i as ActionId; }
//     }
//     (ACTIONS - 1) as ActionId
// }

// // ===== 4) Replay buffer =====

// pub struct ReplayBuffer {
//     buf: VecDeque<(EncodedState, [f32; ACTIONS], f32)>,
//     cap: usize,
// }

// impl ReplayBuffer {
//     pub fn new(cap: usize) -> Self { Self { buf: VecDeque::with_capacity(cap), cap } }
//     pub fn push_game<G: GameAdapter>(&mut self, g: &Trajectory) {
//         // Convert each sample to (state, π, z from that state's player POV)
//         for i in 0..g.encodings.len() {
//             let player = g.players[i] as f32;
//             // If result is from first-player POV, adjust to current state's POV
//             let z = g.result * player; // flip if needed
//             self.push(g.encodings[i].clone(), g.policies[i], z);
//         }
//     }
//     fn push(&mut self, s: EncodedState, pi: [f32; ACTIONS], z: f32) {
//         if self.buf.len() == self.cap { self.buf.pop_front(); }
//         self.buf.push_back((s, pi, z));
//     }
//     pub fn sample_batch(&self, bs: usize, rng: &mut ThreadRng) -> Vec<(EncodedState, [f32; ACTIONS], f32)> {
//         let n = self.buf.len();
//         let mut out = Vec::with_capacity(bs);
//         for _ in 0..bs { let i = rng.gen_range(0..n); out.push(self.buf[i].clone()); }
//         out
//     }
//     pub fn len(&self) -> usize { self.buf.len() }
// }

// // ===== 5) Trainer loop =====

// pub struct TrainCfg {
//     pub batch_size: usize,      // e.g., 512
//     pub steps_per_iter: usize,  // e.g., 1000
//     pub games_per_iter: usize,  // e.g., 50
//     pub replay_size: usize,     // e.g., 100_000
// }

// pub fn train_loop<G: GameAdapter>(
//     mut net: Box<dyn PolicyValueNet>,
//     mcts_cfg: MctsConfig,
//     sp_cfg: SelfPlayCfg,
//     tcfg: TrainCfg,
//     initial_state: G::State,
// ) {
//     let mut rng = rand::thread_rng();
//     let mut replay = ReplayBuffer::new(tcfg.replay_size);
//     let mut best_net = None::<Box<dyn PolicyValueNet>>; // optional evaluation gate

//     for iter in 0.. {
//         // 1) Self-play
//         let mut mcts = Mcts::<G>::new(mcts_cfg.clone(), net.as_ref().into());
//         for _ in 0..tcfg.games_per_iter {
//             let traj = play_one_<G>(&mut mcts, initial_state.clone(), &sp_cfg);
//             replay.push_<G>(&traj);
//         }

//         // 2) Train
//         for step in 0..tcfg.steps_per_iter {
//             let batch = replay.sample_batch(tcfg.batch_size, &mut rng);
//             let (_pl, _vl) = net.train_step(&batch);
//             if step % 100 == 0 { eprintln!("iter {iter}, step {step}, replay {}", replay.len()); }
//         }

//         // 3) (Optional) Evaluate new net vs best and promote
//         if best_net.is_none() { best_net = Some(net.as_ref().into()); }
//         // TODO: implement match_play and promotion threshold here
//     }
// }

// // Helper to coerce &dyn into a Box<dyn> cheaply via trait object clone-like pattern.
// trait IntoBoxedDynNet { fn into(&self) -> Box<dyn PolicyValueNet>; }
// impl<T: PolicyValueNet + Clone + 'static> IntoBoxedDynNet for T {
//     fn into(&self) -> Box<dyn PolicyValueNet> { Box::new(self.clone()) }
// }

// // ===== 6) Example backend stubs =====
// // Implement PolicyValueNet for your chosen framework.

// #[derive(Clone)]
// pub struct DummyNet; // replace with BurnNet, TchNet, etc.
// impl PolicyValueNet for DummyNet {
//     fn predict_batch(&self, batch: &[EncodedState]) -> Vec<NetOut> {
//         batch.iter().map(|_| NetOut { policy_logits: [0.0; ACTIONS], value: 0.0 }).collect()
//     }
//     fn train_step(&mut self, _batch: &[(EncodedState, [f32; ACTIONS], f32)]) -> (f32, f32) { (0.0, 0.0) }
// }

/// Burn network

/// Quoridor AlphaZero-style network.
#[derive(Clone)]
pub struct QuoridorNet
{
    pub device: <NdArray as burn::prelude::Backend>::Device,
    pub network_model: NetworkModel<NdArray>
}

#[derive(Module, Debug)]
pub struct NetworkModel<B: Backend>
{
    pub conv1: Conv2d<B>,
    pub conv2: Conv2d<B>,
    pub fc_policy: nn::Linear<B>,
    pub fc_value1: nn::Linear<B>,
    pub fc_value2: nn::Linear<B>,
}

#[derive(Clone, Debug)]
pub struct NeuralNetOutput<B: Backend> {
    pub policy: Tensor<B, 2>, // [batch, ACTIONS]
    pub value: Tensor<B, 2>,  // [batch, 1]
}

impl QuoridorNet {
    pub fn new() -> Self {
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        let conv_cfg = Conv2dConfig::new([8, 64], [3, 3])
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false }); // in_channels=8, out=64

        let conv1 = conv_cfg.init(&device);

        let conv_cfg2 =
            Conv2dConfig::new([64, 64], [3, 3]).with_initializer(Initializer::KaimingUniform {
                gain: 1.0,
                fan_out_only: false,
            });
        let conv2 = conv_cfg2.init(&device);

        // Flatten feature map (approx 64 * 5 * 5 after two 3x3 conv on 9x9 input, no padding)
        let fc_policy = nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
            .init(&device);

        let fc_value1 = nn::LinearConfig::new(64 * 5 * 5, 64)
            .with_initializer(Initializer::KaimingUniform {
                gain: 1.0,
                fan_out_only: false,
            })
            .init(&device);

        let fc_value2 = nn::LinearConfig::new(64, 1)
            .with_initializer(Initializer::XavierNormal { gain: (1.0) })
            .init(&device);

        Self {
            device,
            network_model: NetworkModel {
                conv1,
                conv2,
                fc_policy,
                fc_value1,
                fc_value2,
            },
        }
    }
    
    /// Create a new network with zero-initialized weights (for testing)
    pub fn new_zero_weights() -> Self {
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        let conv_cfg = Conv2dConfig::new([8, 64], [3, 3])
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 });
        let conv1 = conv_cfg.init(&device);

        let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 });
        let conv2 = conv_cfg2.init(&device);

        let fc_policy = nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);

        let fc_value1 = nn::LinearConfig::new(64 * 5 * 5, 64)
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);

        let fc_value2 = nn::LinearConfig::new(64, 1)
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);

        Self {
            device,
            network_model: NetworkModel { conv1, conv2, fc_policy, fc_value1, fc_value2 }
        }
    }
    
    /// Create a network that prefers upward moves (for testing/verification)
    /// The policy outputs will strongly favor moves 0-3 which are "Up" moves
    pub fn new_biased_upward() -> Self {
        use burn::tensor::{Tensor, TensorData};
        use burn::module::{Module, Param};
        
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        // Create conv layers with zero weights
        let conv_cfg = Conv2dConfig::new([8, 64], [3, 3])
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 });
        let conv1 = conv_cfg.init(&device);

        let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 });
        let conv2 = conv_cfg2.init(&device);

        // Create policy layer with zero weights but biased output
        let fc_policy = nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
            .with_bias(true)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);
        
        // Manually set the bias to favor upward moves
        let mut bias_data = vec![-5.0; ACTIONS]; // Negative bias for most moves
        bias_data[0] = 5.0;  // Up + Up collision - strong positive
        bias_data[1] = 5.0;  // Up + Down collision
        bias_data[2] = 5.0;  // Up + Left collision
        bias_data[3] = 5.0;  // Up + Right collision
        
        let new_bias = Tensor::<NdArray, 1>::from_data(
            TensorData::new(bias_data, [ACTIONS]),
            &device,
        );
        
        // Replace the bias using the record system
        let mut record = fc_policy.into_record();
        record.bias = Some(Param::from_tensor(new_bias));
        let fc_policy = Module::<NdArray>::load_record(
            nn::LinearConfig::new(64 * 5 * 5, ACTIONS).init(&device),
            record
        );

        let fc_value1 = nn::LinearConfig::new(64 * 5 * 5, 64)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);

        let fc_value2 = nn::LinearConfig::new(64, 1)
            .with_initializer(Initializer::Constant { value: 0.0 })
            .init(&device);

        Self {
            device,
            network_model: NetworkModel { conv1, conv2, fc_policy, fc_value1, fc_value2 }
        }
    }
    
    /// Save the network to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
        
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let record = Module::<NdArray>::into_record(self.network_model.clone());
        recorder.record(record, path.as_ref().to_path_buf())
            .map_err(|e| format!("Failed to save: {:?}", e))?;
        Ok(())
    }
    
    /// Load a network from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
        
        let device = <NdArray as burn::prelude::Backend>::Device::default();
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        
        // Load the record
        let record = recorder.load(path.as_ref().to_path_buf(), &device)
            .map_err(|e| format!("Failed to load: {:?}", e))?;
        
        // Create a template network and load the record into it
        let template = Self::new();
        let network_model = Module::<NdArray>::load_record(template.network_model, record);
        
        Ok(Self {
            device,
            network_model,
        })
    }
}

impl<B: Backend> NetworkModel<B>
{
    pub fn forward(&self, x: Tensor<B, 4>) -> NeuralNetOutput<B> {
        let relu = Relu::new();
        // x: [batch, 7, 9, 9]
        let x = self.conv1.forward(x);
        let x = relu.forward(x);
        let x = self.conv2.forward(x);
        let x = relu.forward(x);

        // Flatten: [batch, 64*5*5]
        let x = x.flatten(1, 3);

        // Policy head
        let policy = self.fc_policy.forward(x.clone());

        // Value head
        let value = self.fc_value1.forward(x);
        let value = relu.forward(value);
        let value = self.fc_value2.forward(value).tanh(); // range (-1,1)

        NeuralNetOutput { policy, value }
    }
}

pub fn encode_batch_to_tensor<B: Backend>(
    batch: &[EncodedState],
    device: &B::Device,
) -> Tensor<B, 4> {
    let batch_size = batch.len();
    let c = batch[0].c; // assume all states have the same channel count

    // Flatten into a single Vec<f32>: [batch, c, 9, 9]
    let mut flat: Vec<f32> = Vec::with_capacity(batch_size * c * 9 * 9);

    for state in batch {
        assert_eq!(state.planes.len(), c);
        for chan in 0..c {
            assert_eq!(state.planes[chan].len(), 9);
            for row in 0..9 {
                assert_eq!(state.planes[chan][row].len(), 9);
                flat.extend_from_slice(&state.planes[chan][row]);
            }
        }
    }

    // Build tensor with shape [batch, c, 9, 9]
    Tensor::<B, 4>::from_data(
        burn::tensor::TensorData::new(flat, [batch_size, c, 9, 9]),
        device,
    )
}

fn predict_batch(network: &QuoridorNet, batch: &[EncodedState]) -> Vec<NetOut> {
    // Convert batch &[EncodedState] → Tensor<B,4> of shape [batch, 7, 9, 9]
    let input = encode_batch_to_tensor::<NdArray>(batch, &network.device);

    let out = network.network_model.forward(input);

    // Map NetOut<B> → your NetOut type (convert tensor to Vec<f32>)
    let values: Vec<f32> = out.value.into_data().to_vec().unwrap();

    out.policy
        .iter_dim(0)
        .zip(values)
        .map(|(p, v)| {
            let policy_vec: Vec<f32> = p.into_data().to_vec().unwrap();
            NetOut {
                policy_logits: policy_vec.try_into().expect("Policy wrong length"),
                value: v,
            }
        })
        .collect()
}

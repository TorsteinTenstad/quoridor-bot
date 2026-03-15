// quoridor_az_scaffold.rs
// Minimal, idiomatic Rust scaffold for an AlphaZero-style Quoridor agent.
// Assumes you already have rules/state/move-gen. Plug them in via the GameAdapter trait below.
//
// What you get:
// - A clean Policy+Value network interface (backend-agnostic).
// - MCTS (PUCT) with masking, Dirichlet noise at root, batching hooks for GPU inference.
// - Self-play worker producing (state, π, z) triples.
// - Replay buffer and training loop skeleton.
// - Symmetry augmentation stubs.
//
// You can split this into modules later; kept single-file for clarity.

use burn::backend::NdArray;
use rand::{prelude::*, rng};
use burn;
use burn::nn::{self, Initializer, Relu};
use burn::tensor::{backend::Backend, Tensor};
use burn::module::Module;
use burn::nn::conv::{Conv2d, Conv2dConfig};

use crate::data_model::{Game, Player, PlayerMove, WallOrientation, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, WALL_GRID_HEIGHT, WALL_GRID_WIDTH};
use crate::all_moves::ALL_MOVES;
use crate::game_logic::is_move_legal;


// ===== 0) Domain adapter =====
// Glue layer between YOUR existing rules/state and this scaffold.

/// A compact action id in [0, 138). 0..10 pawn moves, 10..138 walls, for example.
pub type ActionId = u16; // keep it small

/// Encoded input planes for the NN. Shape: C x 9 x 9 flattened to row-major.
#[derive(Clone)]
pub struct EncodedState {
    pub planes: Vec<Vec<Vec<f32>>>, // length = C*9*9
    pub c: usize,         // channels
}

/// Mask of legal actions aligned with the fixed action space.
#[derive(Clone)]
pub struct ActionMask(pub [bool; ACTIONS]);

pub const ACTIONS: usize = 138; // adjust if you use a different scheme


fn action_from_id(action_id: ActionId) -> PlayerMove {
    return ALL_MOVES.get(action_id as usize).unwrap().clone();
}

pub fn get_move(game: &Game, network: &QuoridorNet, player: Player, temperature: f32) -> PlayerMove
{
    let mut rng = rng();

    let prediction = predict_batch(network, &[encode(game)]);

    let legal_moves: Vec<(usize, &f32)> = prediction.first().unwrap().policy_logits.iter().enumerate()
        .filter(|(id, _)|{is_move_legal(game, player, &action_from_id(*id as u16))}).collect();


    // Apply temperature
    let max_logit = legal_moves.iter().map(|&(_, l)| l.clone()).fold(f32::NEG_INFINITY, f32::max);
    let exp_logits: Vec<f32> = legal_moves
        .iter()
        .map(|&(_, logit)| ((logit - max_logit) / temperature).exp())
        .collect();

        // Normalize into probabilities
    let sum_exp: f32 = exp_logits.iter().sum();
    let probs: Vec<f32> = exp_logits.iter().map(|x| x / sum_exp).collect();

    // Sample from distribution
    let dist = rand::distr::weighted::WeightedIndex::new(&probs).unwrap();
    let choice = dist.sample(&mut rng);

    // Extract the most likely move from the output
    action_from_id( legal_moves[choice].0 as u16)
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
                    WallOrientation::Horizontal =>
                        channels[2][y][x] = 1.0,
                    WallOrientation::Vertical =>
                        channels[3][y][x] = 1.0,
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

    EncodedState { planes: channels, c: 8 }
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

// ===== 2) MCTS (PUCT) =====

// #[derive(Clone, Default)]
// struct EdgeStats {
//     n: u32,   // visit count
//     w: f32,   // total value
//     q: f32,   // mean value
//     p: f32,   // prior
// }

// #[derive(Clone, Default)]
// struct Node<G: GameAdapter> {
//     // edges indexed by ActionId; present only for legal actions
//     edges: HashMap<ActionId, EdgeStats>,
//     // cache terminal or expanded
//     expanded: bool,
//     // store mask for quick selection
//     mask: ActionMask,
//     // optional: value estimate at node creation
//     _v0: f32,
//     // store state if you want; we keep only key to save memory in large trees
//     _phantom: std::marker::PhantomData<G>,
// }

// #[derive(Clone)]
// pub struct MctsConfig {
//     pub c_puct: f32,           // ~1.5
//     pub dirichlet_alpha: f32,  // ~0.3
//     pub dirichlet_eps: f32,    // ~0.25
//     pub simulations: usize,    // 200..800
//     pub root_noise: bool,
//     pub temperature: f32,      // for move selection from visits
// }

// impl Default for MctsConfig {
//     fn default() -> Self {
//         Self {
//             c_puct: 1.5,
//             dirichlet_alpha: 0.3,
//             dirichlet_eps: 0.25,
//             simulations: 400,
//             root_noise: true,
//             temperature: 1.0,
//         }
//     }
// }

// pub struct Mcts<G: GameAdapter> {
//     cfg: MctsConfig,
//     net: Box<dyn PolicyValueNet>,
//     // Transposition table: key -> node
//     nodes: HashMap<PositionKey, Node<G>>,
//     rng: ThreadRng,
//     _pd: std::marker::PhantomData<G>,
// }

// impl<G: GameAdapter> Mcts<G> {
//     pub fn new(cfg: MctsConfig, net: Box<dyn PolicyValueNet>) -> Self {
//         Self { cfg, net, nodes: HashMap::new(), rng: rand::thread_rng(), _pd: Default::default() }
//     }

//     fn get_or_expand(&mut self, s: &G::State) -> (PositionKey, bool) {
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

// // ===== 3) Self-play worker =====

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
pub struct QuoridorNet
{
    device: <NdArray as burn::prelude::Backend>::Device,
    network_model: NetworkModel
}

#[derive(Module, Debug, Clone)]
pub struct NetworkModel
{
    conv1: Conv2d<NdArray>,
    conv2: Conv2d<NdArray>,
    fc_policy: nn::Linear<NdArray>,
    fc_value1: nn::Linear<NdArray>,
    fc_value2: nn::Linear<NdArray>,
}

#[derive(Clone, Debug)]
pub struct NeuralNetOutput<B: Backend> {
    pub policy: Tensor<B, 2>, // [batch, 138]
    pub value: Tensor<B, 2>,  // [batch, 1]
}

impl QuoridorNet {
    pub fn new() -> Self {
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        let conv_cfg = Conv2dConfig::new([7, 64], [3, 3])
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false }); // in_channels=7, out=64

        let conv1 = conv_cfg.init(&device);

        let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
          .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
        let conv2 = conv_cfg2.init(&device);

        // Flatten feature map (approx 64 * 5 * 5 after two 3x3 conv on 9x9 input, no padding)
        let fc_policy = nn::LinearConfig::new(64 * 5 * 5, 138)
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
            .init(&device);

        let fc_value1 = nn::LinearConfig::new(64 * 5 * 5, 64)
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
            .init(&device);

        let fc_value2 = nn::LinearConfig::new(64, 1)
            .with_initializer(Initializer::XavierNormal { gain: (1.0) })
            .init(&device);

        Self {
            device,
            network_model: NetworkModel { conv1, conv2, fc_policy, fc_value1, fc_value2 }
        }
    }
}

impl NetworkModel
{
    pub fn forward(&self, x: Tensor<NdArray, 4>) -> NeuralNetOutput<NdArray> {
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

    out.policy.iter_dim(0)
        .zip(values.into_iter())
        .map(|(p, v)| {
            let policy_vec: Vec<f32> = p.into_data().to_vec().unwrap();
            NetOut { policy_logits: policy_vec.try_into().expect("Policy wrong length"), value: v }})
        .collect()
}



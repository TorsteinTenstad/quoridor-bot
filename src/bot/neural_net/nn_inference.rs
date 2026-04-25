use burn::backend::NdArray;
use rand::prelude::*;
use rand::{rng};
use rand::distr::weighted::{WeightedIndex};
use burn;
use burn::nn::{self, Initializer, Relu};
use burn::tensor::{backend::Backend, Tensor};
use burn::module::Module;
use burn::nn::conv::{Conv2d, Conv2dConfig};
use std::path::Path;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::data_model::{Game, Player, PlayerMove, WallOrientation, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, WALL_GRID_HEIGHT, WALL_GRID_WIDTH};
use crate::all_moves::ALL_MOVES;
use crate::game_logic::{is_move_legal, execute_move_unchecked};
use crate::bot::neural_net::nn_config::{MctsConfig, FullTrainingConfig};

pub type ActionId = u16;

/// Total possible actions: 16 pawn moves + 162 wall placements (8x8 grid x 2 orientations + some boundary)
pub const ACTIONS: usize = 178;

/// Encoded input planes for the NN. Shape: C x 9 x 9
#[derive(Clone)]
pub struct EncodedState {
    pub planes: Vec<Vec<Vec<f32>>>,
    pub c: usize, // number of channels
}

/// Mask of legal actions aligned with the fixed action space
#[derive(Clone)]
pub struct ActionMask(pub [bool; ACTIONS]);

pub type PositionKey = u64;

/// Check if the game is over and return the winner
pub fn is_game_over(game: &Game) -> Option<Player> {
    // White wins if reaches y=8
    if game.board.player_position(Player::White).y == 8 {
        return Some(Player::White);
    }
    // Black wins if reaches y=0
    if game.board.player_position(Player::Black).y == 0 {
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
    game.board.player_positions[0].index().hash(&mut hasher);
    game.board.player_positions[1].index().hash(&mut hasher);
    // Hash walls
    for x in 0..WALL_GRID_WIDTH {
        for y in 0..WALL_GRID_HEIGHT {
            if let Some(orientation) = game.board.walls.0[x][y] {
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
        .filter(|(_, mv)| is_move_legal(game, mv))
        .map(|(id, _)| id as ActionId)
        .collect()
}

/// Apply an action to a game state (returns new state)
pub fn apply_action(game: &Game, action_id: ActionId) -> Game {
    let mut new_game = game.clone();
    let player_move = action_from_id(action_id);
    execute_move_unchecked(&mut new_game, &player_move);
    new_game
}

fn action_from_id(action_id: ActionId) -> PlayerMove {
    return ALL_MOVES.get(action_id as usize).unwrap().clone();
}

/// Get a move from the network using temperature-based sampling
pub fn get_move(game: &Game, network: &QuoridorNet, player: Player, temperature: f32) -> PlayerMove
{
    let mut rng = rng();

    let encoded = vec![encode(game)];
    let prediction = predict_batch(network, &encoded);

    let legal_moves: Vec<(usize, &f32)> = prediction.first().unwrap().policy_logits.iter().enumerate()
        .filter(|(id, _)|{is_move_legal(game, &action_from_id(*id as u16))}).collect();

    // Handle edge case of no legal moves (shouldn't happen in valid game)
    if legal_moves.is_empty() {
        panic!("No legal moves available for player {:?}", player);
    }

    // Apply temperature
    let max_logit = legal_moves.iter().map(|&(_, l)| l.clone()).fold(f32::NEG_INFINITY, f32::max);
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
    action_from_id( legal_moves[choice].0 as u16)
}

/// Encode a game state into neural network input format
fn encode(game: &Game) -> EncodedState {
    // shape: [channels, 9, 9]
    let mut channels = vec![vec![vec![0.0; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT]; 6];

    // player pawns - ALWAYS encode current player in channel 0, opponent in channel 1
    // This ensures the network always learns from the current player's perspective
    let current_player = game.player;
    let opponent = current_player.opponent();
    
    let current_pos = game.board.player_position(current_player);
    channels[0][current_pos.y][current_pos.x] = 1.0;
    
    let opponent_pos = game.board.player_position(opponent);
    channels[1][opponent_pos.y][opponent_pos.x] = 1.0;

    // walls (just fill in as 1.0 where a wall is placed)
    for x in 0..WALL_GRID_WIDTH {
        for y in 0..WALL_GRID_HEIGHT {
            if let Some(o) = game.board.walls.0[x][y] {
                match o {
                    WallOrientation::Horizontal =>
                        channels[2][y][x] = 1.0,
                    WallOrientation::Vertical =>
                        channels[3][y][x] = 1.0,
                }
            }
        }
    }

    // walls left (normalized by 10) - current player's walls in channel 4, opponent's in channel 5
    for x in 0..PIECE_GRID_WIDTH {
        for y in 0..PIECE_GRID_HEIGHT {
            channels[4][y][x] = game.walls_left[current_player.as_index()] as f32 / 10.0;
            channels[5][y][x] = game.walls_left[opponent.as_index()] as f32 / 10.0;
        }
    }

    EncodedState { planes: channels, c: 6 }
}

// ===== Network Output =====

/// Output of a network forward pass on a single position
#[derive(Clone)]
pub struct NetOut {
    pub policy_logits: [f32; ACTIONS], // unnormalized logits
    pub value: f32,                    // in [-1, 1]
}

// ===== MCTS Implementation =====

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
pub struct Mcts {
    pub cfg: MctsConfig,
    pub net: QuoridorNet,
    // Transposition table: key -> node
    nodes: HashMap<PositionKey, Node>,
    rng: ThreadRng,
}

impl Mcts {
    pub fn new(cfg: MctsConfig, net: QuoridorNet) -> Self {
        Self { cfg, net, nodes: HashMap::new(), rng: rng() }
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

    /// PUCT formula
    fn puct_score(&self, total_visits: u32, e: &EdgeStats) -> f32 {
        let exploration = self.cfg.c_puct * e.p * ((total_visits as f32).sqrt() / (1.0 + e.n as f32));
        e.q + exploration
    }

    /// Select best action by PUCT
    fn select_action(&self, key: PositionKey) -> Option<ActionId> {
        let node = self.nodes.get(&key)?;
        if node.edges.is_empty() { return None; }

        let total_n: u32 = node.edges.values().map(|e| e.n).sum();
        let (best_action, _score) = node.edges.iter()
            .map(|(&a, e)| (a, self.puct_score(total_n, e)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())?;
        Some(best_action)
    }

    /// Run MCTS from root state, return policy π
    pub fn run(&mut self, state: &Game) -> [f32; ACTIONS] {
        // Expand root if needed
        let (root_key, _) = self.get_or_expand(state);

        // Add Dirichlet noise to root if configured
        if self.cfg.root_noise {
            self.add_dirichlet_noise(root_key);
        }

        // Run simulations (iterative, not recursive, to avoid stack overflow)
        for _ in 0..self.cfg.simulations {
            let mut path: Vec<(PositionKey, ActionId)> = Vec::with_capacity(64);
            let mut current_state = state.clone();
            let mut player_sign = 1.0f32;
            let mut visited_keys = std::collections::HashSet::new();

            // Selection phase
            loop {
                let key = game_to_key(&current_state);
                
                // Check for cycles
                if visited_keys.contains(&key) {
                    // Cycle detected - treat as draw
                    self.backup(&path, 0.0);
                    break;
                }
                visited_keys.insert(key);
                
                // Check if node exists
                if !self.nodes.contains_key(&key) {
                    break;
                }

                // Check for terminal
                if let Some(v) = terminal_value(&current_state) {
                    self.backup(&path, v * player_sign);
                    path.clear();
                    break;
                }

                // Select best action by PUCT
                let action = match self.select_action(key) {
                    Some(a) => a,
                    None => break,
                };
                
                path.push((key, action));
                current_state = apply_action(&current_state, action);
                player_sign = -player_sign;

                // Check if child needs expansion
                let child_key = game_to_key(&current_state);
                if !self.nodes.contains_key(&child_key) {
                    // Expand and evaluate leaf
                    let enc = vec![encode(&current_state)];
                    let out = predict_batch(&self.net, &enc)[0].clone();
                    let legal = legal_action_ids(&current_state);
                    
                    // Softmax over legal actions
                    let logits = out.policy_logits;
                    let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    let mut p = [0f32; ACTIONS];
                    for &a in &legal {
                        let z = (logits[a as usize] - max_logit).exp();
                        p[a as usize] = z;
                        sum += z;
                    }
                    if sum > 0.0 {
                        for &a in &legal {
                            p[a as usize] /= sum;
                        }
                    }
                    
                    let mut edges = HashMap::with_capacity(legal.len());
                    for &a in &legal {
                        edges.insert(a, EdgeStats { n: 0, w: 0.0, q: 0.0, p: p[a as usize] });
                    }
                    
                    self.nodes.insert(child_key, Node { edges, expanded: true, _v0: out.value });
                    
                    // Backup leaf value
                    self.backup(&path, out.value * player_sign);
                    path.clear();
                    break;
                }
            }
        }

        // Extract visit counts as policy
        let mut pi = [0f32; ACTIONS];
        if let Some(node) = self.nodes.get(&root_key) {
            for (&a, e) in &node.edges {
                pi[a as usize] = e.n as f32;
            }
            
            // Apply temperature
            if self.cfg.temperature != 1.0 {
                for x in pi.iter_mut() {
                    *x = x.powf(1.0 / self.cfg.temperature.max(1e-6));
                }
            }
        }

        // Normalize
        let sum: f32 = pi.iter().sum();
        if sum > 0.0 {
            for p in &mut pi { *p /= sum; }
        }

        pi
    }

    /// Backup value along path
    fn backup(&mut self, path: &[(PositionKey, ActionId)], mut v: f32) {
        for (key, a) in path.iter().rev() {
            if let Some(node) = self.nodes.get_mut(key) {
                if let Some(e) = node.edges.get_mut(a) {
                    e.n += 1;
                    e.w += v;
                    e.q = e.w / (e.n as f32);
                }
            }
            v = -v; // Alternate players
        }
    }

    fn add_dirichlet_noise(&mut self, key: PositionKey) {
        use rand_distr::{Gamma, Distribution};

        let node = match self.nodes.get_mut(&key) {
            Some(n) => n,
            None => return,
        };

        if node.edges.is_empty() { return; }
        
        let count = node.edges.len();
        let alpha = self.cfg.dirichlet_alpha;
        let eps = self.cfg.dirichlet_eps;

        // Sample Dirichlet noise
        let gamma_dist = Gamma::new(alpha as f64, 1.0).unwrap();
        let mut noise: Vec<f32> = (0..count).map(|_| gamma_dist.sample(&mut self.rng) as f32).collect();
        let noise_sum: f32 = noise.iter().sum();
        for n in &mut noise { *n /= noise_sum; }

        // Mix with existing priors
        for (edge, &n) in node.edges.values_mut().zip(noise.iter()) {
            edge.p = (1.0 - eps) * edge.p + eps * n;
        }
    }
}

// ===== Neural Network =====

/// Quoridor AlphaZero-style network
#[derive(Clone)]
pub struct QuoridorNet
{
    pub device: <NdArray as burn::prelude::Backend>::Device,
    pub network_model: NetworkModel<NdArray>,
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

        let conv_cfg = Conv2dConfig::new([6, 64], [3, 3])
            .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });

        let conv1 = conv_cfg.init(&device);

        let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
          .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
        let conv2 = conv_cfg2.init(&device);

        // Flatten feature map (approx 64 * 5 * 5 after two 3x3 conv on 9x9 input, no padding)
        let fc_policy = nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
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
    
    /// Create a new network with zero-initialized weights (for testing)
    pub fn new_zero_weights() -> Self {
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        let conv_cfg = Conv2dConfig::new([6, 64], [3, 3])
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
        use burn::module::Param;
        
        let device = <NdArray as burn::prelude::Backend>::Device::default();

        // Create conv layers with zero weights
        let conv_cfg = Conv2dConfig::new([6, 64], [3, 3])
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
    
    /// Save the network to a file (with optional metadata)
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        self.save_with_metadata(path, None)
    }
    
    /// Save the network with training metadata
    pub fn save_with_metadata<P: AsRef<Path>>(
        &self, 
        path: P, 
        config: Option<&FullTrainingConfig>
    ) -> Result<(), Box<dyn std::error::Error>> {
        use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
        
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let record = Module::<NdArray>::into_record(self.network_model.clone());
        recorder.record(record, path.as_ref().to_path_buf())
            .map_err(|e| format!("Failed to save network: {:?}", e))?;
        
        // Save metadata separately if provided
        if let Some(cfg) = config {
            let metadata_path = path.as_ref().with_extension("toml");
            cfg.save_to_file(&metadata_path)?;
        }
        
        Ok(())
    }
    
    /// Load a network from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
        
        let device = <NdArray as burn::prelude::Backend>::Device::default();
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        
        // Load the record
        let record = recorder.load(path.as_ref().to_path_buf(), &device)
            .map_err(|e| format!("Failed to load network: {:?}", e))?;
        
        // Create a template network and load the record into it
        let template = Self::new();
        let network_model = Module::<NdArray>::load_record(template.network_model, record);
        
        Ok(Self {
            device,
            network_model,
        })
    }
    
    /// Load metadata for a network if it exists
    pub fn load_metadata<P: AsRef<Path>>(path: P) -> Result<Option<FullTrainingConfig>, Box<dyn std::error::Error>> {
        let metadata_path = path.as_ref().with_extension("toml");
        if metadata_path.exists() {
            Ok(Some(FullTrainingConfig::load_from_file(&metadata_path)?))
        } else {
            Ok(None)
        }
    }
}

impl<B: Backend> NetworkModel<B>
{
    pub fn forward(&self, x: Tensor<B, 4>) -> NeuralNetOutput<B> {
        let relu = Relu::new();
        // x: [batch, 6, 9, 9]
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

/// Convert batch of encoded states to tensor
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

/// Predict on a batch of encoded states
pub fn predict_batch(network: &QuoridorNet, batch: &[EncodedState]) -> Vec<NetOut> {
    // Convert batch &[EncodedState] → Tensor<B,4> of shape [batch, 6, 9, 9]
    let input = encode_batch_to_tensor::<NdArray>(batch, &network.device);

    let out = network.network_model.forward(input);

    // Map NetOut<B> → NetOut type (convert tensor to Vec<f32>)
    let values: Vec<f32> = out.value.into_data().to_vec().unwrap();

    out.policy.iter_dim(0)
        .zip(values.into_iter())
        .map(|(p, v)| {
            let policy_vec: Vec<f32> = p.into_data().to_vec().unwrap();
            NetOut { policy_logits: policy_vec.try_into().expect("Policy wrong length"), value: v }})
        .collect()
}

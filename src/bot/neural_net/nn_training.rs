use burn::backend::{NdArray, Autodiff};
use burn::tensor::{backend::Backend, Tensor, activation};
use burn::module::{Module, AutodiffModule};
use burn::nn::{Initializer};
use burn::nn::conv::Conv2dConfig;
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use rand::{RngExt, rng};
use rand::prelude::*;
use std::collections::VecDeque;
use std::path::Path;

use crate::data_model::Game;
use crate::bot::neural_net::nn_config::{SelfPlayConfig, FullTrainingConfig, TrainingMetadata};
use crate::bot::neural_net::nn_inference::{
    QuoridorNet, NetworkModel, EncodedState, Mcts, 
    ACTIONS, is_game_over, legal_action_ids, apply_action, ActionId
};

/// Self-play trajectory
pub struct Trajectory {
    pub encodings: Vec<EncodedState>,
    pub policies: Vec<[f32; ACTIONS]>, // π from visits
    pub players: Vec<i8>,              // +1 or -1, whose POV each state was recorded from
    pub result: f32,                   // final z in [-1,1] from first player's POV
}

/// Play one self-play game using MCTS
pub fn play_one_game(mcts: &mut Mcts, initial_state: Game, sp: &SelfPlayConfig) -> Trajectory {
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
        let mut a = sample_from_pi(&pi, &mut rng());
        
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
        encodings.push(encode_state(&current_state));
        
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

/// Helper function to encode game state
fn encode_state(game: &Game) -> EncodedState {
    use crate::data_model::{PIECE_GRID_WIDTH, PIECE_GRID_HEIGHT, WALL_GRID_WIDTH, WALL_GRID_HEIGHT, WallOrientation};
    
    let mut channels = vec![vec![vec![0.0; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT]; 6];

    let current_player = game.player;
    let opponent = current_player.opponent();
    
    let current_pos = game.board.player_position(current_player);
    channels[0][current_pos.y][current_pos.x] = 1.0;
    
    let opponent_pos = game.board.player_position(opponent);
    channels[1][opponent_pos.y][opponent_pos.x] = 1.0;

    for x in 0..WALL_GRID_WIDTH {
        for y in 0..WALL_GRID_HEIGHT {
            if let Some(o) = game.board.walls.0[x][y] {
                match o {
                    WallOrientation::Horizontal => channels[2][y][x] = 1.0,
                    WallOrientation::Vertical => channels[3][y][x] = 1.0,
                }
            }
        }
    }

    for x in 0..PIECE_GRID_WIDTH {
        for y in 0..PIECE_GRID_HEIGHT {
            channels[4][y][x] = game.walls_left[current_player.as_index()] as f32 / 10.0;
            channels[5][y][x] = game.walls_left[opponent.as_index()] as f32 / 10.0;
        }
    }

    EncodedState { planes: channels, c: 6 }
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
    
    let r_val: f32 = rng.random();
    let r = r_val * sum;
    let mut acc = 0.0;
    for (i, p) in pi.iter().enumerate() {
        acc += *p;
        if r <= acc { return i as ActionId; }
    }
    (ACTIONS - 1) as ActionId
}

/// Replay buffer for storing training samples
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
            let i = rng.random_range(0..n);
            out.push(self.buf[i].clone()); 
        }
        out
    }
    
    pub fn len(&self) -> usize { 
        self.buf.len() 
    }
    
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

/// Create a new NetworkModel on any backend
pub fn create_network_model<B: Backend>(device: &B::Device) -> NetworkModel<B> {
    let conv_cfg = Conv2dConfig::new([6, 64], [3, 3])
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
    let conv1 = conv_cfg.init(device);

    let conv_cfg2 = Conv2dConfig::new([64, 64], [3, 3])
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false });
    let conv2 = conv_cfg2.init(device);

    let fc_policy = burn::nn::LinearConfig::new(64 * 5 * 5, ACTIONS)
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
        .init(device);

    let fc_value1 = burn::nn::LinearConfig::new(64 * 5 * 5, 64)
        .with_initializer(Initializer::KaimingUniform { gain: 1.0, fan_out_only: false })
        .init(device);

    let fc_value2 = burn::nn::LinearConfig::new(64, 1)
        .with_initializer(Initializer::XavierNormal { gain: 1.0 })
        .init(device);

    NetworkModel { conv1, conv2, fc_policy, fc_value1, fc_value2 }
}

/// Main AlphaZero-style training loop
pub fn train_loop(
    initial_net: &QuoridorNet,
    config: &FullTrainingConfig,
    save_path: Option<&Path>,
) {
    let mcts_cfg = config.mcts.clone();
    let sp_cfg = config.self_play.clone();
    let tcfg = config.training.clone();
    
    let mut rng = rng();
    let mut replay = ReplayBuffer::new(tcfg.replay_size);
    
    // Create network on autodiff backend for training
    type ADBackend = Autodiff<NdArray>;
    let device = <ADBackend as Backend>::Device::default();
    
    let mut train_model = create_network_model::<ADBackend>(&device);
    
    let mut optim = AdamConfig::new()
        .with_epsilon(1e-7)
        .init();

    let start_time = chrono::Utc::now();
    
    println!("Starting training loop...");
    println!("Config: {} iterations, {} games/iter, {} steps/iter", 
             tcfg.iterations, tcfg.games_per_iter, tcfg.steps_per_iter);
    println!("Learning rate: {}\n", tcfg.learning_rate);

    let mut final_policy_loss = 0.0;
    let mut final_value_loss = 0.0;
    let mut total_games = 0;

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
            total_games += 1;
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
            
            final_policy_loss = total_policy_loss / (tcfg.steps_per_iter as f32);
            final_value_loss = total_value_loss / (tcfg.steps_per_iter as f32);
            println!("  Average losses - Policy: {:.4}, Value: {:.4}", final_policy_loss, final_value_loss);
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
                
                // Create metadata
                let metadata = TrainingMetadata {
                    created_at: start_time.to_rfc3339(),
                    completed_at: if iter + 1 == tcfg.iterations { 
                        Some(chrono::Utc::now().to_rfc3339()) 
                    } else { 
                        None 
                    },
                    iterations_completed: iter + 1,
                    total_games_played: total_games,
                    final_policy_loss,
                    final_value_loss,
                };
                
                let config_with_metadata = config.clone().with_metadata(metadata);
                
                save_net.save_with_metadata(&checkpoint_path, Some(&config_with_metadata))
                    .expect("Failed to save checkpoint");
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
        
        let metadata = TrainingMetadata {
            created_at: start_time.to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            iterations_completed: tcfg.iterations,
            total_games_played: total_games,
            final_policy_loss,
            final_value_loss,
        };
        
        let final_config = config.clone().with_metadata(metadata);
        
        final_net.save_with_metadata(path, Some(&final_config))
            .expect("Failed to save final model");
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
    
    // Encode batch inputs
    let batch_size = batch.len();
    let c = 6; // channels
    
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
    
    // Convert targets to tensors
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
    
    // Return loss values for logging
    let policy_inner = policy_loss.clone().inner();
    let value_inner = value_loss.clone().inner();
    let policy_loss_val: f32 = policy_inner.into_data().to_vec().unwrap()[0];
    let value_loss_val: f32 = value_inner.into_data().to_vec().unwrap()[0];
    
    (policy_loss_val, value_loss_val)
}

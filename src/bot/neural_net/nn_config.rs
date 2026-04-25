use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// Configuration for MCTS (Monte Carlo Tree Search)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MctsConfig {
    pub c_puct: f32,           // Exploration constant (~1.5)
    pub dirichlet_alpha: f32,  // Dirichlet noise parameter (~0.3)
    pub dirichlet_eps: f32,    // Dirichlet noise weight (~0.25)
    pub simulations: usize,    // Number of simulations per move (200..800)
    pub root_noise: bool,      // Whether to add noise at root
    pub temperature: f32,      // Temperature for move selection from visits
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

/// Configuration for self-play games
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfPlayConfig {
    pub sims_per_move: usize,     // MCTS simulations per move
    pub temperature_moves: usize, // Number of moves to use temperature=1.0
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        Self {
            sims_per_move: 400,
            temperature_moves: 10,
        }
    }
}

/// Configuration for training
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub batch_size: usize,       // Training batch size
    pub steps_per_iter: usize,   // Gradient steps per iteration
    pub games_per_iter: usize,   // Self-play games per iteration
    pub replay_size: usize,      // Replay buffer size
    pub iterations: usize,       // Total training iterations
    pub learning_rate: f64,      // Adam learning rate
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            batch_size: 128,
            steps_per_iter: 100,
            games_per_iter: 10,
            replay_size: 10_000,
            iterations: 100,
            learning_rate: 0.001,
        }
    }
}

/// Complete training configuration with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullTrainingConfig {
    pub mcts: MctsConfig,
    pub self_play: SelfPlayConfig,
    pub training: TrainingConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TrainingMetadata>,
}

impl Default for FullTrainingConfig {
    fn default() -> Self {
        Self {
            mcts: MctsConfig::default(),
            self_play: SelfPlayConfig::default(),
            training: TrainingConfig::default(),
            metadata: None,
        }
    }
}

/// Metadata about a trained model
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingMetadata {
    pub created_at: String,           // Timestamp when training started
    pub completed_at: Option<String>, // Timestamp when training completed
    pub iterations_completed: usize,  // Number of iterations completed
    pub total_games_played: usize,    // Total self-play games
    pub final_policy_loss: f32,       // Final policy loss
    pub final_value_loss: f32,        // Final value loss
}

impl FullTrainingConfig {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let toml = toml::to_string_pretty(self)?;
        fs::write(path, toml)?;
        Ok(())
    }
    
    /// Merge with command-line overrides
    pub fn with_overrides(
        mut self,
        iterations: Option<usize>,
        games_per_iter: Option<usize>,
        sims_per_move: Option<usize>,
        learning_rate: Option<f64>,
    ) -> Self {
        if let Some(iter) = iterations {
            self.training.iterations = iter;
        }
        if let Some(games) = games_per_iter {
            self.training.games_per_iter = games;
        }
        if let Some(sims) = sims_per_move {
            self.mcts.simulations = sims;
            self.self_play.sims_per_move = sims;
        }
        if let Some(lr) = learning_rate {
            self.training.learning_rate = lr;
        }
        self
    }
    
    /// Add metadata to the configuration
    pub fn with_metadata(mut self, metadata: TrainingMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Helper to create a default config file
pub fn create_default_config_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let config = FullTrainingConfig::default();
    config.save_to_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_config_serialization() {
        let config = FullTrainingConfig::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        println!("Default config:\n{}", toml);
        
        let parsed: FullTrainingConfig = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.training.iterations, 100);
    }
    
    #[test]
    fn test_config_file_roundtrip() {
        let test_path = "test_config.toml";
        let config = FullTrainingConfig::default();
        
        config.save_to_file(test_path).unwrap();
        let loaded = FullTrainingConfig::load_from_file(test_path).unwrap();
        
        assert_eq!(loaded.training.iterations, config.training.iterations);
        assert_eq!(loaded.mcts.c_puct, config.mcts.c_puct);
        
        fs::remove_file(test_path).unwrap();
    }
}

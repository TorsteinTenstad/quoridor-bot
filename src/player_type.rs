use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap_derive::ValueEnum)]
pub enum PlayerType {
    Human,
    Bot,
    NeuralNet
}

impl Display for PlayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerType::Human => write!(f, "human"),
            PlayerType::Bot => write!(f, "bot"),
            PlayerType::NeuralNet => write!(f, "neural network")
        }
    }
}

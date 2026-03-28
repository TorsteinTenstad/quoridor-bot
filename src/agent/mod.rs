pub mod bot;
pub mod nn_bot;
pub mod random;

use std::fmt::Display;

use clap::ValueEnum;

use crate::{
    commands::Session,
    data_model::{Game, PlayerMove},
};

pub trait Agent {
    type Command;

    fn get_move(&mut self, game: &Game) -> PlayerMove;

    fn name(&self) -> &str {
        "agent"
    }

    fn execute(&mut self, _session: &mut Session, _cmd: Self::Command) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap_derive::ValueEnum)]
pub enum AgentArg {
    Bot,
    Manual,
    NeuralNet,
    Random,
}

impl Display for AgentArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.to_possible_value()
                .expect("clap ValueEnum unable to find its own name")
                .get_name(),
        )
    }
}

pub enum InputType {
    Manual,
    Automatic(AgentType),
}

pub enum AgentType {
    Bot,
    NeuralNet,
    Random(random::Random),
}

impl Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputType::Manual => write!(f, "manual"),
            InputType::Automatic(agent) => match agent {
                AgentType::Bot => write!(f, "bot"),
                AgentType::NeuralNet => write!(f, "neural network"),
                AgentType::Random(agent) => write!(f, "{}", agent.name()),
            },
        }
    }
}

impl From<AgentArg> for InputType {
    fn from(value: AgentArg) -> InputType {
        match value {
            AgentArg::Manual => InputType::Manual,
            AgentArg::Bot => InputType::Automatic(AgentType::Bot),
            AgentArg::NeuralNet => InputType::Automatic(AgentType::NeuralNet),
            AgentArg::Random => InputType::Automatic(AgentType::Random(random::Random)),
        }
    }
}

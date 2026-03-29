pub mod abe;
pub mod carlo;
pub mod dedi;
pub mod neural_net;
pub mod random;

use crate::{
    commands::Session,
    data_model::{Game, PlayerMove},
};

pub trait Agent: Default {
    type Command;

    fn get_move(&mut self, game: &Game) -> PlayerMove;
    fn execute(&mut self, _session: &mut Session, _cmd: Self::Command) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap_derive::ValueEnum)]
pub enum AgentType {
    Abe,
    Carlo,
    NeuralNet,
    Random,
}

#[derive(Default)]
pub struct Agents {
    abe: abe::Abe,
    carlo: carlo::Carlo,
    neural_net: neural_net::NeuralNet,
    random: random::Random,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum BotCommand {
    Abe(abe::AbeCommand),
    Carlo(carlo::CarloCommand),
    Random(random::RandomCommand),
}

impl Agents {
    pub fn get_move(&mut self, game: &Game, agent_type: &AgentType) -> PlayerMove {
        match agent_type {
            AgentType::Abe => self.abe.get_move(game),
            AgentType::Carlo => self.carlo.get_move(game),
            AgentType::NeuralNet => self.neural_net.get_move(game),
            AgentType::Random => self.random.get_move(game),
        }
    }

    pub fn execute_bot_command(&mut self, session: &mut Session, command: BotCommand) {
        match command {
            BotCommand::Carlo(cmd) => self.carlo.execute(session, cmd.cmd),
            BotCommand::Random(cmd) => self.random.execute(session, cmd.cmd),
            BotCommand::Abe(cmd) => self.abe.execute(session, cmd.cmd),
        }
    }
}

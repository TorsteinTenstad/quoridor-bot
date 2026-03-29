pub mod abe;
pub mod carlo;
pub mod dedi;
pub mod neural_net;
pub mod random;

use crate::{
    data_model::{Game, PlayerMove},
    session::Session,
};

pub trait Bot: Default {
    type Command;

    fn get_move(&mut self, game: &Game) -> PlayerMove;
    fn execute(&mut self, _session: &mut Session, _cmd: Self::Command) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap_derive::ValueEnum)]
pub enum BotType {
    Abe,
    Carlo,
    NeuralNet,
    Random,
}

#[derive(Default)]
pub struct Bots {
    abe: abe::Abe,
    carlo: carlo::Carlo,
    neural_net: neural_net::NeuralNet,
    random: random::Random,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum BotCommand {
    #[command(subcommand)]
    Abe(abe::AbeCommand),
    #[command(subcommand)]
    Carlo(carlo::CarloCommand),
    #[command(subcommand)]
    NeuralNet(neural_net::NeuralNetCommand),
    #[command(subcommand)]
    Random(random::RandomCommand),
}

impl Bots {
    pub fn get_move(&mut self, game: &Game, bot_type: &BotType) -> PlayerMove {
        match bot_type {
            BotType::Abe => self.abe.get_move(game),
            BotType::Carlo => self.carlo.get_move(game),
            BotType::NeuralNet => self.neural_net.get_move(game),
            BotType::Random => self.random.get_move(game),
        }
    }

    pub fn execute_bot_command(&mut self, session: &mut Session, command: BotCommand) {
        match command {
            BotCommand::Carlo(cmd) => self.carlo.execute(session, cmd),
            BotCommand::Random(cmd) => self.random.execute(session, cmd),
            BotCommand::NeuralNet(cmd) => self.neural_net.execute(session, cmd),
            BotCommand::Abe(cmd) => self.abe.execute(session, cmd),
        }
    }
}

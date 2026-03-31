use crate::{
    args::Args,
    data_model::{Game, PlayerMove},
    session::Session,
};

pub struct NeuralNet {
    default_temperature: f32,
}

impl NeuralNet {
    pub fn init(&mut self, args: &Args) {
        self.default_temperature = args.temperature;
    }
}

impl Default for NeuralNet {
    fn default() -> Self {
        Self {
            default_temperature: 0.5,
        }
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum NeuralNetCommand {
    Move {
        #[arg(short, long, default_value_t = 0.5)]
        temperature: f32,
    },
}

impl super::Bot for NeuralNet {
    type Command = NeuralNetCommand;

    fn get_move(&mut self, _game: &Game) -> PlayerMove {
        unreachable!()
    }

    fn execute(&mut self, _session: &mut Session, _cmd: Self::Command) {}
}

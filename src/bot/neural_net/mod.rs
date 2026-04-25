use std::path::PathBuf;

use crate::{args::Args, bot::neural_net::nn_bot::{QuoridorNet, get_move}, data_model::{Game, Player, PlayerMove}, session::Session};

mod nn_bot;
mod nn_config;
mod nn_inference;
mod nn_training;


impl NeuralNet {
    pub fn init(&mut self, args: &Args, player: &Player) {
        self.temperature = args.temperature;
        let path = match player {
            Player::Black => args.b_nn_path.as_ref(),
            Player::White => args.w_nn_path.as_ref()
        };
        if let Some(path) = path
        {
           self.net = QuoridorNet::load(path).unwrap();
        }
    }
}

pub struct NeuralNet {
    net: nn_bot::QuoridorNet,
    temperature: f32,
}

impl Default for NeuralNet {
    fn default() -> Self {
        Self {
            net: nn_bot::QuoridorNet::new(),
            temperature: 0.5,
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

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        get_move(game, &self.net, self.temperature)
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            Self::Command::Move { temperature } => {
                session.make_move(get_move(&session.game, &self.net, temperature))
            }
        }
    }
}
           

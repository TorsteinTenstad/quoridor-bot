pub mod minimax;
pub mod walls;

use std::time::Duration;

use crate::{
    args,
    bot::{Bot, dedi::walls::get_move},
    data_model::{Game, PlayerMove},
    session::Session,
};

#[derive(Default)]
pub struct Dedi {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
}

impl Dedi {
    pub fn load_default_params(&mut self, args: &args::Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
    }
}

impl Bot for Dedi {
    type Command = DediCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let duration = self
            .default_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3));
        minimax::minimax_iterative(game, duration).unwrap()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            DediCommand::Walls => get_move(&session.game),
        }
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum DediCommand {
    Walls,
}

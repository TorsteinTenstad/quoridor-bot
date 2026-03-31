pub mod monte;
use crate::{
    args::{self, DEFAULT_DURATION},
    bot::Bot,
    data_model::{Game, PlayerMove},
    session::Session,
};
use std::time::Duration;

#[derive(Default)]
pub struct Monte {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
}

impl Monte {
    pub fn init(&mut self, args: &args::Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
    }
}

impl Bot for Monte {
    type Command = MonteCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let duration = self
            .default_seconds
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_DURATION);

        monte::monte(game, duration)
    }

    fn execute(&mut self, _session: &mut Session, _cmd: Self::Command) {}
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum MonteCommand {}

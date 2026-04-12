pub mod heuristic;
pub mod minimax;
pub mod walls;
use std::time::Duration;

use crate::{
    args::{self, DEFAULT_DURATION},
    bot::{
        Bot,
        dedi::{heuristic::Heuristic, minimax::Cache},
    },
    data_model::{Game, Player, PlayerMove},
    session::Session,
};

#[derive(Default)]
pub struct Dedi {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
    default_heuristics: [Heuristic; 2],
    cache: Cache,
}

impl Dedi {
    pub fn init(&mut self, args: &args::Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
        if let Some(h) = &args.dedi_heuristic_white {
            self.default_heuristics[Player::White.as_index()] = h.clone();
        }
        if let Some(h) = &args.dedi_heuristic_black {
            self.default_heuristics[Player::Black.as_index()] = h.clone();
        }
    }
}

impl Bot for Dedi {
    type Command = DediCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let duration = self
            .default_seconds
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_DURATION);
        let h = &self.default_heuristics[game.player.as_index()];

        let mut game = game.clone();
        minimax::minimax_iterative(&mut game, h, duration, &mut self.cache).unwrap()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            DediCommand::Move { seconds, heuristic } => {
                let duration = seconds.map(Duration::from_secs).unwrap_or(DEFAULT_DURATION);
                let h = heuristic
                    .as_ref()
                    .unwrap_or(&self.default_heuristics[session.game.player.as_index()]);
                let mut game = session.game.clone();

                let m =
                    minimax::minimax_iterative(&mut game, h, duration, &mut self.cache).unwrap();
                session.make_move(m);
            }
        }
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum DediCommand {
    Move {
        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,

        #[arg(short, long)]
        heuristic: Option<Heuristic>,
    },
}

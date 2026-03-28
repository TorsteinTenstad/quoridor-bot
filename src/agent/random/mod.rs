use core::time;
use std::thread::sleep;

use crate::{
    agent::Agent,
    all_moves::ALL_MOVES,
    commands::{Command, Session, execute_command},
    data_model::{Game, PlayerMove},
};

pub struct Random;

impl Agent for Random {
    type Command = SubCommand;

    fn get_move(&mut self, _game: &Game) -> PlayerMove {
        // TODO: random valid move
        ALL_MOVES[0].clone()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            SubCommand::Move { seconds } => {
                sleep(time::Duration::from_secs(seconds));

                let game = session.game_states.last().unwrap();
                let m = self.get_move(game);
                execute_command(session, Command::PlayMove(m));
            }
        }
    }
}

#[derive(clap_derive::Parser, Debug)]
pub struct RandomCommand {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum SubCommand {
    Move {
        #[arg(short, long, default_value_t = 0)]
        seconds: u64,
    },
}

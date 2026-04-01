mod bfs;
mod board;
mod mcts;
mod node;

use crate::{
    bot::Bot,
    data_model::{Game, PlayerMove},
    session::Session,
};

#[derive(Default)]
pub struct Carlo {
    mcst: mcts::Mcts,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum CarloCommand {
    Move,
    DebugBoard,
}

impl Bot for Carlo {
    type Command = CarloCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        self.mcst.get_move(game)
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            CarloCommand::Move => session.make_move(self.get_move(&session.game)),
            CarloCommand::DebugBoard => {
                println!("{:?}", board::Board::from(&session.game))
            }
        }
    }
}

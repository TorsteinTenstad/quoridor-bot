pub mod minimax;
pub mod walls;

use crate::{
    bot::{Bot, dedi::walls::get_move},
    data_model::{Game, PlayerMove},
    session::Session,
};

#[derive(Default)]
pub struct Dedi {}

impl Bot for Dedi {
    type Command = DediCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let (m, _) = minimax::minimax(game, 5);
        m.unwrap()
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

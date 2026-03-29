mod walls;

use crate::{
    bot::{Bot, dedi::walls::get_move},
    data_model::{Direction, Game, MovePiece, PlayerMove},
    session::Session,
};

#[derive(Default)]
pub struct Dedi {}

impl Bot for Dedi {
    type Command = DediCommand;

    fn get_move(&mut self, _: &Game) -> PlayerMove {
        PlayerMove::MovePiece({
            MovePiece {
                direction: Direction::Down,
                direction_on_collision: Direction::Down,
            }
        })
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

pub mod bit_set;
mod walls;

use crate::{
    agent::{Agent, dedi::walls::get_illegal_walls},
    commands::Session,
    data_model::{Direction, Game, MovePiece, PlayerMove},
};

#[derive(Default)]
pub struct Dedi {}

impl Agent for Dedi {
    type Command = SubCommand;

    fn name(&self) -> &str {
        "dedi"
    }

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
            SubCommand::D1 => {
                get_illegal_walls(session.game_states.last().unwrap());
            }
        }
    }
}

#[derive(clap_derive::Parser, Debug)]
pub struct DediCommand {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum SubCommand {
    D1,
}

mod walls;

use crate::{
    agent::Agent,
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

    fn execute(&mut self, _: &mut Session, _: Self::Command) {}
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum SubCommand {}

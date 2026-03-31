mod bfs;
mod board;
mod mcts;
mod node;

use crate::{
    bot::Bot,
    commands::parse_player_move,
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
    PlaceWall { m: String },
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
            CarloCommand::PlaceWall { m } => {
                // if let Some(PlayerMove::PlaceWall {
                //     orientation,
                //     position,
                // }) = parse_player_move(&m)
                // {
                //     let mut board = board::Board::from(&session.game);
                //     println!("{:?}", board);
                //     board.recalculate_bfs(position.x, position.y, orientation);
                //     println!("{:?}", board)
                // }
            }
        }
    }
}

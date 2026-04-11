mod bfs;
mod board;
mod buffer;
mod iter;
mod mcts;
mod node;

use crate::{
    args::{self, DEFAULT_DURATION},
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
    DebugBoard,
    WallValid { m: String },
}

impl Carlo {
    pub fn init(&mut self, args: &args::Args) {
        self.mcst.default_seconds = args.seconds.unwrap_or(DEFAULT_DURATION.as_secs());
    }
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
                let board = board::Board::from(&session.game);
                let bfs_w = board.bfs_white.printable(&session.game);
                println!("{:?}", bfs_w);
            }
            CarloCommand::WallValid { m } => {
                if let Some(PlayerMove::PlaceWall {
                    orientation,
                    position,
                }) = parse_player_move(&m)
                {
                    let board = board::Board::from(&session.game);
                    let bfs_w = board.bfs_white.printable(&session.game);
                    let bfs_b = board.bfs_black.printable(&session.game);

                    let v = board.valid_wall(position.x, position.y, orientation);
                    println!("{:?}\n", bfs_w);
                    println!("{:?}\n{}", bfs_b, v.0);
                }
            }
        }
    }
}

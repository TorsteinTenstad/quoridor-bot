mod board;
mod path;

use rand::{rngs::ThreadRng, seq::IteratorRandom};

use crate::{
    all_moves::ALL_MOVES,
    bot::Bot,
    commands::parse_player_move,
    data_model::{Game, PlayerMove},
    game_logic::is_move_legal,
    session::Session,
};

#[derive(Default)]
pub struct Carlo {
    rng: ThreadRng,
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
        ALL_MOVES
            .iter()
            .filter(|m| is_move_legal(game, m))
            .choose(&mut self.rng)
            .expect("at least one move will always be valid")
            .clone()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            CarloCommand::Move => session.make_move(self.get_move(&session.game)),
            CarloCommand::DebugBoard => {
                println!("{:?}", board::Board::from(&session.game))
            }
            CarloCommand::PlaceWall { m } => {
                if let Some(PlayerMove::PlaceWall {
                    orientation,
                    position,
                }) = parse_player_move(&m)
                {
                    let mut board = board::Board::from(&session.game);
                    println!("{:?}", board);
                    board.place_wall(position.x, position.y, orientation);
                    println!("{:?}", board)
                }
            }
        }
    }
}

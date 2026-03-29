mod board;

use rand::{rngs::ThreadRng, seq::IteratorRandom};

use crate::{
    agent::Agent,
    all_moves::ALL_MOVES,
    commands::{Session, parse_player_move},
    data_model::{Game, PlayerMove},
    game_logic::{execute_move_unchecked, is_move_legal},
};

#[derive(Default)]
pub struct Carlo {
    rng: ThreadRng,
}

impl Agent for Carlo {
    type Command = SubCommand;

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
            SubCommand::Move => {
                let game = session.game_states.last().unwrap();
                let m = self.get_move(game);
                let game = execute_move_unchecked(game, &m);
                session.push(game, m)
            }
            SubCommand::DebugBoard => {
                let game = session.game_states.last().unwrap();
                let board = board::Board::from(game);
                println!("{:?}", board)
            }
            SubCommand::PlaceWall { m } => {
                if let Some(PlayerMove::PlaceWall {
                    orientation,
                    position,
                }) = parse_player_move(&m)
                {
                    let game = session.game_states.last().unwrap();
                    let mut board = board::Board::from(game);
                    board.place_wall(position.x, position.y, orientation);
                    println!("{:?}", board)
                }
            }
        }
    }
}

#[derive(clap_derive::Parser, Debug)]
pub struct CarloCommand {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum SubCommand {
    Move,
    PlaceWall { m: String },
    DebugBoard,
}

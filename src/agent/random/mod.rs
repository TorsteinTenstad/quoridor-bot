use core::time;
use std::thread::sleep;

use rand::{rngs::ThreadRng, seq::IteratorRandom};

use crate::{
    agent::Agent,
    commands::{Command, Session, execute_command},
    data_model::{Direction, Game, MovePiece, PlayerMove},
    game_logic::is_move_direction_legal_with_player_at_position,
};

#[derive(Default)]
pub struct Random {
    rng: ThreadRng,
}

impl Agent for Random {
    type Command = SubCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let pos = game.board.player_position(game.player);
        let dir = Direction::iter()
            .filter(|d| is_move_direction_legal_with_player_at_position(&game.board, pos, d))
            .choose(&mut self.rng)
            .expect("at least one move will always be valid");
        PlayerMove::MovePiece(MovePiece {
            direction: dir,
            direction_on_collision: Direction::Up,
        })
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

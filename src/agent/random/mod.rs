use core::time;
use std::thread::sleep;

use rand::{Rng, rngs::ThreadRng, seq::IteratorRandom};

use crate::{
    agent::Agent,
    all_moves::ALL_MOVES,
    commands::{Command, Session, execute_command},
    data_model::{Game, PlayerMove},
    game_logic::{all_move_piece_moves, is_move_legal_with_player_at_position},
};

#[derive(Default)]
pub struct Random {
    rng: ThreadRng,
}

impl Agent for Random {
    type Command = SubCommand;

    fn name(&self) -> &str {
        "random"
    }

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let pos = game.board.player_position(game.player);
        let opponent_pos = game.board.player_position(game.player.opponent());
        let moves: Box<dyn Iterator<Item = PlayerMove>> = if self.rng.random::<f32>() < 0.75 {
            Box::new(all_move_piece_moves(pos, opponent_pos).map(|m| PlayerMove::MovePiece(m)))
        } else {
            Box::new(ALL_MOVES.iter().cloned())
        };

        moves
            .filter(|m| is_move_legal_with_player_at_position(&game, game.player, pos, m))
            .choose(&mut self.rng)
            .expect("at least one move will always be valid")
            .clone()
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

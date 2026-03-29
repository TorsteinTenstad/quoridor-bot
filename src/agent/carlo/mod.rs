use rand::{rngs::ThreadRng, seq::IteratorRandom};

use crate::{
    agent::Agent,
    all_moves::ALL_MOVES,
    commands::{Command, Session, execute_command},
    data_model::{Game, PlayerMove},
    game_logic::is_move_legal_with_player_at_position,
};

#[derive(Default)]
pub struct Carlo {
    rng: ThreadRng,
}

impl Agent for Carlo {
    type Command = SubCommand;

    fn name(&self) -> &str {
        "carlo"
    }

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let pos = game.board.player_position(game.player);
        ALL_MOVES
            .iter()
            .filter(|m| is_move_legal_with_player_at_position(&game, game.player, pos, m))
            .choose(&mut self.rng)
            .expect("at least one move will always be valid")
            .clone()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            SubCommand::Move => {
                let game = session.game_states.last().unwrap();
                let m = self.get_move(game);
                execute_command(session, Command::PlayMove(m));
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
}

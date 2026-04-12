use crate::{
    args::{self, DEFAULT_DURATION},
    bot::{
        darwin::data_model::EvaluatedPopulation,
        dedi::{
            heuristic::Heuristic,
            minimax::{self, Cache},
        },
    },
    data_model::{Game, Player, PlayerMove},
    generic_heuristic::GenericHeuristicWeights,
    session::Session,
};
use std::{fs::File, path::PathBuf, time::Duration};

pub mod data_model;
pub mod evaluate;
pub mod evolve;

#[derive(Default)]
pub struct Darwin {
    default_seconds: Option<u64>,
    default_weights: [GenericHeuristicWeights; 2],
    cache: Cache,
}

impl Darwin {
    pub fn init(&mut self, args: &args::Args) {
        self.default_seconds = args.seconds;
        load(
            &args.darwin_weights_white,
            &mut self.default_weights,
            Player::White,
        );
        load(
            &args.darwin_weights_black,
            &mut self.default_weights,
            Player::Black,
        );
    }

    pub fn get_move(&mut self, game: &Game) -> PlayerMove {
        let duration = self
            .default_seconds
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_DURATION);
        let h = Heuristic::Generic(self.default_weights[game.player.as_index()].clone());

        let mut game = game.clone();
        minimax::minimax_iterative(&mut game, &h, duration, &mut self.cache).unwrap()
    }

    pub fn get_move_fixed_depth(&mut self, game: &Game) -> PlayerMove {
        let h = Heuristic::Generic(self.default_weights[game.player.as_index()].clone());
        let mut game = game.clone();
        minimax::minimax(&mut game, 3, &h, None, &mut self.cache)
            .unwrap()
            .0
            .unwrap()
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum DarwinCommand {
    Move {
        #[arg(short, long)]
        seconds: Option<u64>,

        #[arg(short, long)]
        weights: Option<PathBuf>,
    },
}

impl super::Bot for Darwin {
    type Command = DarwinCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        self.get_move(game)
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            Self::Command::Move { seconds, weights } => {
                let duration = seconds.map(Duration::from_secs).unwrap_or(DEFAULT_DURATION);
                let weights = match weights {
                    Some(path) => {
                        let file = match File::open(path) {
                            Ok(w) => w,
                            Err(e) => {
                                println!("{}", e);
                                return;
                            }
                        };
                        match serde_json::from_reader::<_, GenericHeuristicWeights>(file) {
                            Ok(w) => w,
                            Err(e) => {
                                println!("{}", e);
                                return;
                            }
                        }
                    }
                    None => self.default_weights[session.game.player.as_index()].clone(),
                };
                let h = Heuristic::Generic(weights);

                let mut game = session.game.clone();
                let m =
                    minimax::minimax_iterative(&mut game, &h, duration, &mut self.cache).unwrap();
                session.make_move(m);
            }
        }
    }
}

fn load(path: &Option<PathBuf>, weights: &mut [GenericHeuristicWeights; 2], player: Player) {
    let Some(path) = path else { return };
    let population =
        serde_json::from_reader::<_, EvaluatedPopulation>(File::open(path).unwrap()).unwrap();
    let best = population
        .0
        .iter()
        .max_by(|a, b| a.win_rate.total_cmp(&b.win_rate))
        .unwrap();
    println!("Darwin ({}) loaded\n{:?}", player.to_string(), &best.genes);
    weights[player.as_index()] = best.genes.clone();
}

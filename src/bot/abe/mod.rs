use crate::{
    args::{Args, DEFAULT_DURATION},
    bot::{
        Bot,
        abe::{
            alpha_beta::{
                BoardEvaluation, Cache, WHITE_LOSES_BLACK_WINS, WHITE_WINS_BLACK_LOSES,
                best_move_alpha_beta, best_move_alpha_beta_iterative_deepening,
            },
            heuristic::Heuristic,
            move_ordering::moves_ordered_by_heuristic_quality,
        },
    },
    commands::parse_player_move,
    data_model::{Game, PlayerMove},
    game_logic::{execute_move_unchecked, execute_move_unchecked_inplace, is_move_legal},
    l_p_a_star::Pathfinding,
    session::Session,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
pub mod alpha_beta;
pub mod heuristic;
pub mod move_ordering;

#[derive(Default)]
pub struct Abe {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
    default_heuristic: Heuristic,
    min_depth_for_caching: usize,
    game_state: Arc<Mutex<Game>>,
    cache: Cache,
    workers: Vec<JoinHandle<()>>,
    flags: Vec<Arc<AtomicBool>>,
}

impl Abe {
    pub fn init(&mut self, args: &Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
        self.min_depth_for_caching = args.min_depth_for_caching;
        if let Some(heuristic) = args.heuristic {
            self.default_heuristic = heuristic;
        }
        self.update_game_state(Game::new());
        for i in 0..args.abe_background_threads {
            let n = args.abe_background_threads;
            let game_state = Arc::clone(&self.game_state);
            let cache = self.cache.clone();
            let heuristic = self.default_heuristic;
            let min_depth_for_caching = self.min_depth_for_caching;
            self.flags.push(Default::default());
            let flag = self.flags.iter().last().unwrap().clone();
            self.workers.push(std::thread::spawn(move || {
                worker(
                    i,
                    n,
                    game_state,
                    cache,
                    heuristic,
                    min_depth_for_caching,
                    flag,
                )
            }));
        }
    }
    pub fn update_game_state(&mut self, game: Game) {
        *self.game_state.lock().unwrap() = game;
        for flag in &self.flags {
            flag.store(true, Ordering::Release);
        }
    }
    pub fn clear_cache(&mut self) {
        self.cache.transposition_table.lock().unwrap().clear();
    }
}

fn worker(
    i: usize,
    n: usize,
    game_state: Arc<Mutex<Game>>,
    mut cache: Cache,
    heuristic: Heuristic,
    min_depth_for_caching: usize,
    stop_flag: Arc<AtomicBool>,
) -> () {
    let mut currently_working_on = game_state.lock().unwrap().clone();
    let mut depth = 0;
    let stop = || stop_flag.swap(false, Ordering::Acquire);
    loop {
        depth += 1;
        for player_move in moves_ordered_by_heuristic_quality(&currently_working_on)
            .iter()
            .skip(i)
            .step_by(n)
        {
            let child_game_state = execute_move_unchecked(&currently_working_on, player_move);
            let mut pathfinding = Pathfinding::new(&child_game_state.board);
            if pathfinding.any_blocked(&child_game_state.board) {
                continue;
            }
            alpha_beta::alpha_beta(
                &child_game_state,
                depth,
                WHITE_LOSES_BLACK_WINS,
                WHITE_WINS_BLACK_LOSES,
                &[],
                &stop,
                heuristic,
                &mut cache,
                min_depth_for_caching,
                &mut pathfinding,
            );
            let potentially_new = game_state.lock().unwrap().clone();
            if potentially_new != currently_working_on {
                currently_working_on = potentially_new;
                depth = 0;
                break;
            }
        }
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum AbeCommand {
    Move {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,

        #[arg(short, long)]
        heuristic: Option<Heuristic>,
    },
    Show {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,

        #[arg(short, long)]
        heuristic: Option<Heuristic>,
    },
    Eval {
        #[arg()]
        move_to_evaluate: Option<String>,

        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,

        #[arg(short, long)]
        heuristic: Option<Heuristic>,

        #[arg(short, long)]
        verbose: bool,

        #[arg(short = 'o', long)]
        show_outcome: bool,
    },
    Heuristic {
        heuristic: Option<Heuristic>,
    },
    ClearCache,
}

impl Bot for Abe {
    type Command = AbeCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        self.update_game_state(game.clone());
        let (duration, eval) = get_bot_move(
            game,
            self.default_depth,
            self.default_seconds.map(Duration::from_secs),
            self.default_heuristic,
            &mut self.cache,
            self.min_depth_for_caching,
        );
        let depth = eval.best_moves.len();
        let m = eval.best_moves.into_iter().last().unwrap();
        print!("{}", m);
        print!(" score:{}", eval.score);
        print!(" depth:{}", depth);
        println!(" {:?}", duration);
        self.update_game_state(execute_move_unchecked(game, &m));
        m
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        self.update_game_state(session.game.clone());
        match cmd {
            AbeCommand::Show {
                depth,
                seconds,
                heuristic,
            } => {
                let (duration, eval) = get_bot_move(
                    &session.game,
                    depth,
                    seconds.map(Duration::from_secs),
                    heuristic.unwrap_or(self.default_heuristic),
                    &mut self.cache,
                    self.min_depth_for_caching,
                );
                println!("{eval} {:?}", duration);
            }
            AbeCommand::Move {
                depth,
                seconds,
                heuristic,
            } => {
                let (duration, eval) = get_bot_move(
                    &session.game,
                    depth,
                    seconds.map(Duration::from_secs),
                    heuristic.unwrap_or(self.default_heuristic),
                    &mut self.cache,
                    self.min_depth_for_caching,
                );
                print!("{}", eval.best_moves.last().unwrap());
                print!(" score:{}", eval.score);
                print!(" depth:{}", eval.best_moves.len());
                println!(" {:?}", duration);
                let m = eval.best_moves.into_iter().last().unwrap();
                self.update_game_state(execute_move_unchecked(&session.game, &m));
                session.make_move(m);
            }
            AbeCommand::Eval {
                move_to_evaluate,
                depth,
                seconds,
                heuristic,
                verbose,
                show_outcome,
            } => {
                let initial = match move_to_evaluate {
                    Some(move_str) => match parse_player_move(&move_str) {
                        Some(m) if is_move_legal(&session.game, &m) => Some(m),
                        Some(_) => {
                            println!("Illegal move");
                            return;
                        }
                        None => {
                            println!("Could not parse move: {}", move_str);
                            return;
                        }
                    },
                    None => None,
                };
                let mut game = session.game.clone();
                if let Some(m) = &initial {
                    execute_move_unchecked_inplace(&mut game, m)
                }
                let (duration, eval) = get_bot_move(
                    &game,
                    depth,
                    seconds.map(Duration::from_secs),
                    heuristic.unwrap_or(self.default_heuristic),
                    &mut self.cache,
                    self.min_depth_for_caching,
                );
                let move_name = initial
                    .as_ref()
                    .map(PlayerMove::to_string)
                    .unwrap_or("Best move".into());
                println!("{} evaluates to {}", move_name, eval.score);
                if verbose {
                    println!("{eval} {:?}", duration);
                }
                if show_outcome {
                    let n = eval.best_moves.len() + initial.is_some() as usize;
                    let moves = initial.into_iter().chain(eval.best_moves.into_iter().rev());
                    for m in moves {
                        session.make_move(m);
                    }
                    println!("Showing outcome. Use `undo {n}` to revert")
                }
            }
            AbeCommand::Heuristic { heuristic } => {
                let heuristic = heuristic.unwrap_or_default();
                let val = heuristic.eval(
                    &session.game,
                    &mut Pathfinding::new(&session.game.board),
                    true,
                );
                println!("{:?}:{}", heuristic, val);
            }
            AbeCommand::ClearCache => self.clear_cache(),
        }
    }
}

pub fn get_bot_move(
    game: &Game,
    depth: Option<usize>,
    duration: Option<Duration>,
    heuristic: Heuristic,
    cache: &mut Cache,
    min_depth_for_caching: usize,
) -> (Duration, BoardEvaluation) {
    let start_time = std::time::Instant::now();
    let best_moves = match (depth, duration) {
        (Some(depth), _) => {
            best_move_alpha_beta(game, depth, heuristic, cache, min_depth_for_caching)
        }
        (_, duration) => {
            let duration = duration.unwrap_or(DEFAULT_DURATION);
            best_move_alpha_beta_iterative_deepening(
                game,
                duration,
                heuristic,
                cache,
                min_depth_for_caching,
            )
        }
    };
    (start_time.elapsed(), best_moves)
}

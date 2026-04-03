use crate::{
    args::{Args, DEFAULT_DURATION},
    bot::{Bot, abe::heuristic::Heuristic},
    commands::parse_player_move,
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, execute_move_unchecked_inplace,
        is_move_legal, is_move_piece_legal, room_for_wall_placement,
    },
    l_p_a_star::Pathfinding,
    session::Session,
    square_outline_iterator::SquareOutlineIterator,
};
use std::{
    collections::HashMap,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub mod heuristic;

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
            let child_game_state = execute_move_unchecked(&currently_working_on, &player_move);
            let mut pathfinding = Pathfinding::new(&child_game_state.board);
            if pathfinding.any_blocked(&child_game_state.board) {
                continue;
            }
            alpha_beta(
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

pub const WHITE_LOSES_BLACK_WINS: isize = isize::MIN + 1;
pub const WHITE_WINS_BLACK_LOSES: isize = -WHITE_LOSES_BLACK_WINS;

pub fn best_move_alpha_beta_iterative_deepening(
    game: &Game,
    search_duration: Duration,
    heuristic: Heuristic,
    cache: &mut Cache,
    min_depth_for_caching: usize,
) -> BoardEvaluation {
    let deadline = Instant::now() + search_duration;
    let stop = || Instant::now() > deadline;
    let mut eval: BoardEvaluation = Default::default();
    let mut depth = 0;
    let mut pathfinding = Pathfinding::new(&game.board);
    loop {
        match alpha_beta(
            game,
            depth + 1,
            WHITE_LOSES_BLACK_WINS,
            WHITE_WINS_BLACK_LOSES,
            &eval.best_moves,
            &stop,
            heuristic,
            cache,
            min_depth_for_caching,
            &mut pathfinding,
        ) {
            AlphaBetaResult::Stopped => {
                break eval;
            }
            AlphaBetaResult::Moves(moves) => {
                eval = moves;
                depth += 1;
            }
        }
    }
}
pub fn best_move_alpha_beta(
    game: &Game,
    depth: usize,
    heuristic: Heuristic,
    cache: &mut Cache,
    min_depth_for_caching: usize,
) -> BoardEvaluation {
    let mut pathfinding = Pathfinding::new(&game.board);
    match alpha_beta(
        game,
        depth,
        WHITE_LOSES_BLACK_WINS,
        WHITE_WINS_BLACK_LOSES,
        Default::default(),
        &|| false,
        heuristic,
        cache,
        min_depth_for_caching,
        &mut pathfinding,
    ) {
        AlphaBetaResult::Stopped => unreachable!(),
        AlphaBetaResult::Moves(moves) => moves,
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardEvaluation {
    pub score: isize,
    pub best_moves: Vec<PlayerMove>,
}

impl Display for BoardEvaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.best_moves.last().unwrap())?;
        write!(f, " score:{}", self.score)?;
        write!(f, " depth:{}", self.best_moves.len())?;
        write!(
            f,
            " (full chain: {})",
            self.best_moves
                .iter()
                .rev()
                .map(|m| format!("{m};"))
                .collect::<String>()
        )?;
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct Cache {
    transposition_table: Arc<Mutex<HashMap<u64, BoardEvaluation>>>,
}

fn hash_to_u64(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
enum AlphaBetaResult {
    Moves(BoardEvaluation),
    Stopped,
}

#[allow(clippy::too_many_arguments)]
fn alpha_beta<F>(
    game: &Game,
    depth: usize,
    alpha: isize,
    beta: isize,
    search_first: &[PlayerMove],
    stop: &F,
    heuristic: Heuristic,
    cache: &mut Cache,
    min_depth_for_caching: usize,
    pathfinding: &mut Pathfinding,
) -> AlphaBetaResult
where
    F: Fn() -> bool,
{
    if stop() {
        return AlphaBetaResult::Stopped;
    }
    let game_hash = hash_to_u64(game);
    let search_first = match cache.transposition_table.lock().unwrap().get(&game_hash) {
        Some(eval) => {
            if eval.best_moves.len() >= depth {
                return AlphaBetaResult::Moves(eval.clone());
            } else if eval.best_moves.len() > search_first.len() {
                &eval.best_moves.clone()
            } else {
                search_first
            }
        }
        None => search_first,
    };
    if depth == 0
        || game.board.player_position(Player::White).y == PIECE_GRID_HEIGHT - 1
        || game.board.player_position(Player::Black).y == 0
    {
        let heuristic_board_score = heuristic.eval(game, pathfinding, false);
        return AlphaBetaResult::Moves(BoardEvaluation {
            score: heuristic_board_score,
            best_moves: Default::default(),
        });
    }
    let (last, rest) = search_first
        .split_last()
        .map(|(last, rest)| (Some(last), rest))
        .unwrap_or((None, &[]));
    let mut alpha = alpha;
    let mut beta = beta;
    let mut best_moves = Vec::new();
    let player = game.player;
    match player {
        Player::White => {
            let mut value = WHITE_LOSES_BLACK_WINS;
            for player_move in last
                .cloned()
                .into_iter()
                .chain(moves_ordered_by_heuristic_quality(game))
            {
                let child_game_state = execute_move_unchecked(game, &player_move);
                let mut next_pathfinding =
                    pathfinding.clone_with_move(&child_game_state.board, &player_move);
                if next_pathfinding.any_blocked(&child_game_state.board) {
                    continue;
                }
                let (score, moves) = match alpha_beta(
                    &child_game_state,
                    depth - 1,
                    alpha,
                    beta,
                    if Some(&player_move) == last {
                        rest
                    } else {
                        &[]
                    },
                    stop,
                    heuristic,
                    cache,
                    min_depth_for_caching,
                    &mut next_pathfinding,
                ) {
                    AlphaBetaResult::Moves(eval) => (eval.score, eval.best_moves),
                    AlphaBetaResult::Stopped => {
                        return AlphaBetaResult::Stopped;
                    }
                };
                if score > value || best_moves.is_empty() {
                    best_moves = moves;
                    best_moves.push(player_move);
                }
                value = isize::max(value, score);
                if value >= beta {
                    break;
                }
                alpha = isize::max(alpha, value);
            }
            let eval = BoardEvaluation {
                score: value,
                best_moves,
            };
            if depth >= min_depth_for_caching {
                let game_hash = hash_to_u64(game);
                let mut t = cache.transposition_table.lock().unwrap();
                if t.get(&game_hash)
                    .is_none_or(|cache_eval| cache_eval.best_moves.len() < depth)
                {
                    t.insert(game_hash, eval.clone());
                }
            }
            AlphaBetaResult::Moves(eval)
        }
        Player::Black => {
            let mut value = WHITE_WINS_BLACK_LOSES;
            for player_move in last
                .cloned()
                .into_iter()
                .chain(moves_ordered_by_heuristic_quality(game))
            {
                let child_game_state = execute_move_unchecked(game, &player_move);
                let mut next_pathfinding =
                    pathfinding.clone_with_move(&child_game_state.board, &player_move);
                if next_pathfinding.any_blocked(&child_game_state.board) {
                    continue;
                }
                let (score, moves) = match alpha_beta(
                    &child_game_state,
                    depth - 1,
                    alpha,
                    beta,
                    if Some(&player_move) == last {
                        rest
                    } else {
                        &[]
                    },
                    stop,
                    heuristic,
                    cache,
                    min_depth_for_caching,
                    &mut next_pathfinding,
                ) {
                    AlphaBetaResult::Moves(eval) => (eval.score, eval.best_moves),
                    AlphaBetaResult::Stopped => {
                        return AlphaBetaResult::Stopped;
                    }
                };
                if score < value || best_moves.is_empty() {
                    best_moves = moves;
                    best_moves.push(player_move);
                }
                value = isize::min(value, score);
                if value <= alpha {
                    break;
                }
                beta = isize::min(beta, value);
            }
            let eval = BoardEvaluation {
                score: value,
                best_moves,
            };
            if depth >= min_depth_for_caching {
                let game_hash = hash_to_u64(game);
                let mut t = cache.transposition_table.lock().unwrap();
                if t.get(&game_hash)
                    .is_none_or(|cache_eval| cache_eval.best_moves.len() < depth)
                {
                    t.insert(game_hash, eval.clone());
                }
            }
            AlphaBetaResult::Moves(eval)
        }
    }
}

fn moves_ordered_by_heuristic_quality(game: &Game) -> Vec<PlayerMove> {
    let player_position = game.board.player_position(game.player);
    let opponent_position = game.board.player_position(game.player.opponent());

    let mut moves: Vec<PlayerMove> = all_move_piece_moves(player_position, opponent_position)
        .filter(move |move_piece| is_move_piece_legal(game, move_piece))
        .map(PlayerMove::MovePiece)
        .collect();
    if game.walls_left[game.player.as_index()] > 0 {
        let origin = opponent_position;
        for i in 1.. {
            let top_left_x = origin.x as isize - i as isize;
            let top_left_y = origin.y as isize - i as isize;
            let side_length = 2 * i;
            let mut some_in_bounds = false;
            for (x, y) in SquareOutlineIterator::new(top_left_x, top_left_y, side_length) {
                let in_bounds = x >= 0
                    && y >= 0
                    && x < WALL_GRID_WIDTH as isize
                    && y < WALL_GRID_HEIGHT as isize;
                if !in_bounds {
                    continue;
                }
                some_in_bounds = true;
                for orientation in [WallOrientation::Horizontal, WallOrientation::Vertical] {
                    let player_move = PlayerMove::PlaceWall {
                        orientation,
                        position: WallPosition {
                            x: x as usize,
                            y: y as usize,
                        },
                    };
                    if room_for_wall_placement(&game.board.walls, orientation, x, y) {
                        moves.push(player_move);
                    }
                }
            }
            if !some_in_bounds {
                break;
            }
        }
    }
    moves
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

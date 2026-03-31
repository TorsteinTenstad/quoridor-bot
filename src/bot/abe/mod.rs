use crate::{
    args::Args,
    bot::{Bot, abe::heuristic::Heuristic},
    commands::parse_player_move,
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, WALL_GRID_HEIGHT, WALL_GRID_WIDTH,
        WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_legal, is_move_piece_legal,
        room_for_wall_placement,
    },
    l_p_a_star::Pathfinding,
    session::Session,
    square_outline_iterator::SquareOutlineIterator,
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    time::{Duration, Instant},
};
pub mod heuristic;

#[derive(Default)]
pub struct Abe {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
    default_heuristic: Heuristic,
    cache: Cache,
}

impl Abe {
    pub fn load_default_params(&mut self, args: &Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
        if let Some(heuristic) = args.heuristic {
            self.default_heuristic = heuristic;
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
    },
    Heuristic {
        heuristic: Option<Heuristic>,
    },
    ExportCache {
        #[arg()]
        file: PathBuf,
    },
    ImportCache {
        #[arg()]
        file: PathBuf,
    },
}

impl Bot for Abe {
    type Command = AbeCommand;

    fn get_move(&mut self, game: &Game) -> PlayerMove {
        let (_, eval) = get_bot_move(
            game,
            self.default_depth,
            self.default_seconds.map(Duration::from_secs),
            self.default_heuristic,
            &mut self.cache,
        );
        eval.best_moves.into_iter().last().unwrap()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
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
                );
                print!("{}", eval.best_moves.last().unwrap());
                print!(" score:{}", eval.score);
                print!(" depth:{}", eval.best_moves.len());
                println!(" {:?}", duration);
                let m = eval.best_moves.into_iter().last().unwrap();
                session.make_move(m)
            }
            AbeCommand::Eval {
                move_to_evaluate,
                depth,
                seconds,
                heuristic,
            } => {
                if let Some(move_str) = move_to_evaluate {
                    if let Some(m) = parse_player_move(&move_str) {
                        if is_move_legal(&session.game, &m) {
                            let next_game_state = execute_move_unchecked(&session.game, &m);
                            let (duration, eval) = get_bot_move(
                                &next_game_state,
                                depth,
                                seconds.map(Duration::from_secs),
                                heuristic.unwrap_or(self.default_heuristic),
                                &mut self.cache,
                            );
                            println!("{} {:?}", eval.score, duration);
                        } else {
                            println!("Invalid move");
                        }
                    } else {
                        println!("Could not parse move: {}", move_str);
                    }
                } else {
                    let (_, eval) = get_bot_move(
                        &session.game,
                        depth,
                        seconds.map(Duration::from_secs),
                        heuristic.unwrap_or(self.default_heuristic),
                        &mut self.cache,
                    );
                    println!("Best move evaluates to {}", eval.score);
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
            AbeCommand::ExportCache { file: path } => match std::fs::File::create(path) {
                Ok(file) => {
                    serde_json::ser::to_writer_pretty(file, &self.cache).unwrap();
                }
                Err(e) => {
                    println!("{:?}", e)
                }
            },
            AbeCommand::ImportCache { file: path } => match std::fs::File::open(path) {
                Ok(file) => match serde_json::de::from_reader::<_, Cache>(file) {
                    Ok(cache) => self.cache = cache,
                    Err(e) => {
                        println!("{:?}", e)
                    }
                },
                Err(e) => {
                    println!("{:?}", e)
                }
            },
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
) -> BoardEvaluation {
    let deadline = Some(Instant::now() + search_duration);
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
            deadline,
            heuristic,
            cache,
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
) -> BoardEvaluation {
    let mut pathfinding = Pathfinding::new(&game.board);
    match alpha_beta(
        game,
        depth,
        WHITE_LOSES_BLACK_WINS,
        WHITE_WINS_BLACK_LOSES,
        Default::default(),
        None,
        heuristic,
        cache,
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

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    #[serde(with = "serde_json_any_key::any_key_map")]
    transposition_table: HashMap<Game, BoardEvaluation>,
}
enum AlphaBetaResult {
    Moves(BoardEvaluation),
    Stopped,
}

#[allow(clippy::too_many_arguments)]
fn alpha_beta(
    game: &Game,
    depth: usize,
    alpha: isize,
    beta: isize,
    search_first: &[PlayerMove],
    deadline: Option<Instant>,
    heuristic: Heuristic,
    cache: &mut Cache,
    pathfinding: &mut Pathfinding,
) -> AlphaBetaResult {
    if deadline.is_some_and(|deadline| Instant::now() > deadline) {
        return AlphaBetaResult::Stopped;
    }
    let search_first = match cache.transposition_table.get(game) {
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
                let mut pathfinding =
                    pathfinding.clone_with_move(&child_game_state.board, &player_move);
                if pathfinding.any_blocked(&child_game_state.board) {
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
                    deadline,
                    heuristic,
                    cache,
                    &mut pathfinding,
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
            if depth > 1
                && cache
                    .transposition_table
                    .get(game)
                    .is_none_or(|eval| eval.best_moves.len() < depth)
            {
                cache.transposition_table.insert(game.clone(), eval.clone());
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
                let mut pathfinding =
                    pathfinding.clone_with_move(&child_game_state.board, &player_move);
                if pathfinding.any_blocked(&child_game_state.board) {
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
                    deadline,
                    heuristic,
                    cache,
                    &mut pathfinding,
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
            if depth > 1
                && cache
                    .transposition_table
                    .get(game)
                    .is_none_or(|eval| eval.best_moves.len() < depth)
            {
                cache.transposition_table.insert(game.clone(), eval.clone());
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
) -> (Duration, BoardEvaluation) {
    let start_time = std::time::Instant::now();
    let best_moves = match (depth, duration) {
        (Some(depth), _) => best_move_alpha_beta(game, depth, heuristic, cache),
        (_, duration) => {
            let duration = duration.unwrap_or(Duration::from_secs(5));
            best_move_alpha_beta_iterative_deepening(game, duration, heuristic, cache)
        }
    };
    (start_time.elapsed(), best_moves)
}

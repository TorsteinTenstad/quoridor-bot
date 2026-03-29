use crate::{
    a_star::a_star,
    a_star_to_opponent::a_star_to_opponent,
    args::Args,
    bot::Bot,
    commands::parse_player_move,
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, TOTAL_WALLS, WALL_GRID_HEIGHT,
        WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_legal, is_move_piece_legal,
        room_for_wall_placement,
    },
    render_board,
    session::Session,
    square_outline_iterator::SquareOutlineIterator,
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Abe {
    default_depth: Option<usize>,
    default_seconds: Option<u64>,
    cache: Cache,
}

impl Abe {
    pub fn load_default_params(&mut self, args: &Args) {
        self.default_depth = args.depth;
        self.default_seconds = args.seconds;
    }
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum AbeCommand {
    Move {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
    },
    ShowMove {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
    },
    Eval {
        #[arg()]
        move_to_evaluate: Option<String>,

        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
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
        let (_, best_moves) = get_bot_move(
            game,
            self.default_depth,
            self.default_seconds.map(Duration::from_secs),
            &mut self.cache,
        );
        best_moves.into_iter().last().unwrap().best_move
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        match cmd {
            AbeCommand::ShowMove { depth, seconds } => {
                let (_, best_moves) = get_bot_move(
                    &session.game,
                    depth,
                    seconds.map(Duration::from_secs),
                    &mut self.cache,
                );
                for eval in best_moves.iter().rev() {
                    println!("{eval}");
                }
            }
            AbeCommand::Move { depth, seconds } => {
                let (duration, best_moves) = get_bot_move(
                    &session.game,
                    depth,
                    seconds.map(Duration::from_secs),
                    &mut self.cache,
                );
                let m = best_moves.into_iter().last().unwrap().best_move;
                println!("{} {:?}", m, duration);
                session.make_move(m)
            }
            AbeCommand::Eval {
                move_to_evaluate,
                depth,
                seconds,
            } => {
                if let Some(move_str) = move_to_evaluate {
                    if let Some(m) = parse_player_move(&move_str) {
                        if is_move_legal(&session.game, &m) {
                            let next_game_state = execute_move_unchecked(&session.game, &m);
                            let (_, best_moves) = get_bot_move(
                                &next_game_state,
                                depth,
                                seconds.map(Duration::from_secs),
                                &mut self.cache,
                            );
                            println!("{}", best_moves.last().unwrap().score);
                        } else {
                            println!("Invalid move");
                        }
                    } else {
                        println!("Could not parse move: {}", move_str);
                    }
                } else {
                    let (_, best_moves) = get_bot_move(
                        &session.game,
                        depth,
                        seconds.map(Duration::from_secs),
                        &mut self.cache,
                    );
                    println!(
                        "Best move evaluates to {}",
                        best_moves.last().unwrap().score
                    );
                }
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

pub fn heuristic_board_score(game: &Game) -> isize {
    let black_path = a_star(&game.board, Player::Black);
    let white_path = a_star(&game.board, Player::White);
    if white_path.is_none() {
        println!(
            "{:?} has no path in the following board:\n{}",
            Player::White,
            render_board::render_board(&game.board)
        );
    }
    let black_distance = black_path.unwrap().len() as isize;
    if black_distance == 0 {
        return WHITE_LOSES_BLACK_WINS;
    }
    let white_distance = white_path.unwrap().len() as isize;
    if white_distance == 0 {
        return WHITE_WINS_BLACK_LOSES;
    }
    let white_walls_left = game.walls_left[Player::White.as_index()] as isize;
    let black_walls_left = game.walls_left[Player::Black.as_index()] as isize;
    let distance_score = black_distance - white_distance;
    let wall_score = white_walls_left - black_walls_left;
    let total_walls_played = TOTAL_WALLS
        - game.walls_left[Player::White.as_index()]
        - game.walls_left[Player::Black.as_index()];
    let wall_progress = total_walls_played as f32 / TOTAL_WALLS as f32;
    let wall_value = 75.0 - 50.0 * wall_progress;
    let (distance_priority, wall_priority) = (100, wall_value);

    let path_length_between_players = a_star_to_opponent(&game.board, game.player)
        .map(|v| v.len())
        .unwrap_or(usize::MAX);

    let side = (game.board.player_position(Player::White).y as f32
        + game.board.player_position(Player::Black).y as f32)
        / (PIECE_GRID_HEIGHT - 1) as f32
        - 1.0;

    let side_component = -side * 1000.0 / path_length_between_players as f32;

    distance_priority * distance_score
        + (wall_priority * wall_score as f32 + side_component) as isize
}

pub fn best_move_alpha_beta_iterative_deepening(
    game: &Game,
    search_duration: Duration,
    cache: &mut Cache,
) -> Vec<BoardEvaluation> {
    let deadline = Some(Instant::now() + search_duration);
    let mut best_moves: Vec<BoardEvaluation> = Default::default();
    let mut depth = 0;
    loop {
        let search_first = best_moves
            .iter()
            .map(|eval| eval.best_move.clone())
            .collect::<Vec<_>>();
        match alpha_beta(
            game,
            depth + 1,
            WHITE_LOSES_BLACK_WINS,
            WHITE_WINS_BLACK_LOSES,
            &search_first,
            deadline,
            cache,
        ) {
            AlphaBetaResult::Stopped => {
                break best_moves;
            }
            AlphaBetaResult::Moves((_, moves)) => {
                best_moves = moves;
                depth += 1;
            }
        }
    }
}
pub fn best_move_alpha_beta(game: &Game, depth: usize, cache: &mut Cache) -> Vec<BoardEvaluation> {
    match alpha_beta(
        game,
        depth,
        WHITE_LOSES_BLACK_WINS,
        WHITE_WINS_BLACK_LOSES,
        Default::default(),
        None,
        cache,
    ) {
        AlphaBetaResult::Stopped => unreachable!(),
        AlphaBetaResult::Moves((_, moves)) => moves,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardEvaluation {
    pub score: isize,
    pub best_move: PlayerMove,
    pub depth: usize,
}

impl Display for BoardEvaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.best_move)?;
        write!(f, " score:{}", self.score)?;
        write!(f, " depth:{}", self.depth)?;
        Ok(())
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    #[serde(with = "serde_json_any_key::any_key_map")]
    transposition_table: HashMap<Game, Vec<BoardEvaluation>>,
}
enum AlphaBetaResult {
    Moves((isize, Vec<BoardEvaluation>)),
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
    cache: &mut Cache,
) -> AlphaBetaResult {
    if deadline.is_some_and(|deadline| Instant::now() > deadline) {
        return AlphaBetaResult::Stopped;
    }
    let search_first = match cache.transposition_table.get(game) {
        Some(evaluations) => match evaluations.last() {
            Some(eval) => {
                if eval.depth >= depth {
                    return AlphaBetaResult::Moves((eval.score, evaluations.clone()));
                } else if search_first.len() <= evaluations.len() {
                    &evaluations
                        .iter()
                        .map(|eval| eval.best_move.clone())
                        .collect::<Vec<_>>()
                } else {
                    search_first
                }
            }
            None => search_first,
        },
        None => search_first,
    };
    let heuristic_board_score = heuristic_board_score(game);
    if depth == 0
        || heuristic_board_score == WHITE_LOSES_BLACK_WINS
        || heuristic_board_score == WHITE_WINS_BLACK_LOSES
    {
        return AlphaBetaResult::Moves((heuristic_board_score, Default::default()));
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
                if a_star(&child_game_state.board, player).is_none()
                    || a_star(&child_game_state.board, player.opponent()).is_none()
                {
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
                    cache,
                ) {
                    AlphaBetaResult::Moves(moves) => moves,
                    AlphaBetaResult::Stopped => {
                        return AlphaBetaResult::Stopped;
                    }
                };
                if score > value || best_moves.is_empty() {
                    best_moves = moves;
                    best_moves.push(BoardEvaluation {
                        score,
                        best_move: player_move,
                        depth,
                    });
                }
                value = isize::max(value, score);
                if value >= beta {
                    break;
                }
                alpha = isize::max(alpha, value);
            }
            if cache
                .transposition_table
                .get(game)
                .is_none_or(|t| t.last().is_none_or(|t| t.depth < depth))
            {
                cache
                    .transposition_table
                    .insert(game.clone(), best_moves.clone());
            }
            AlphaBetaResult::Moves((value, best_moves))
        }
        Player::Black => {
            let mut value = WHITE_WINS_BLACK_LOSES;
            for player_move in last
                .cloned()
                .into_iter()
                .chain(moves_ordered_by_heuristic_quality(game))
            {
                let child_game_state = execute_move_unchecked(game, &player_move);
                if a_star(&child_game_state.board, player).is_none()
                    || a_star(&child_game_state.board, player.opponent()).is_none()
                {
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
                    cache,
                ) {
                    AlphaBetaResult::Moves(moves) => moves,
                    AlphaBetaResult::Stopped => {
                        return AlphaBetaResult::Stopped;
                    }
                };
                if score < value || best_moves.is_empty() {
                    best_moves = moves;
                    best_moves.push(BoardEvaluation {
                        score,
                        best_move: player_move,
                        depth,
                    });
                }
                value = isize::min(value, score);
                if value <= alpha {
                    break;
                }
                beta = isize::min(beta, value);
            }
            if cache
                .transposition_table
                .get(game)
                .is_none_or(|t| t.last().is_none_or(|t| t.depth < depth))
            {
                cache
                    .transposition_table
                    .insert(game.clone(), best_moves.clone());
            }
            AlphaBetaResult::Moves((value, best_moves))
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
    cache: &mut Cache,
) -> (Duration, Vec<BoardEvaluation>) {
    let start_time = std::time::Instant::now();
    let best_moves = match (depth, duration) {
        (Some(depth), _) => best_move_alpha_beta(game, depth, cache),
        (_, duration) => {
            let duration = duration.unwrap_or(Duration::from_secs(3));
            best_move_alpha_beta_iterative_deepening(game, duration, cache)
        }
    };
    (start_time.elapsed(), best_moves)
}

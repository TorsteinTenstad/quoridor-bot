use crate::{
    a_star::a_star,
    a_star_to_opponent::a_star_to_opponent,
    agent::Agent,
    commands::{Session, parse_player_move},
    data_model::{
        Game, PIECE_GRID_HEIGHT, Player, PlayerMove, TOTAL_WALLS, WALL_GRID_HEIGHT,
        WALL_GRID_WIDTH, WallOrientation, WallPosition,
    },
    game_logic::{
        all_move_piece_moves, execute_move_unchecked, is_move_legal,
        is_move_piece_legal_with_player_at_position, room_for_wall_placement,
    },
    render_board,
    square_outline_iterator::SquareOutlineIterator,
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Abe {}

impl Agent for Abe {
    type Command = SubCommand;

    fn get_move(&mut self, _game: &Game) -> PlayerMove {
        todo!()
    }

    fn execute(&mut self, session: &mut Session, cmd: Self::Command) {
        let current_game_state = session.game_states.last().unwrap();
        let player = current_game_state.player;
        match cmd {
            SubCommand::ShowMove { depth, seconds } => {
                let (_, best_moves) = get_bot_move(
                    current_game_state,
                    player,
                    depth,
                    seconds.map(Duration::from_secs),
                    &mut session.cache,
                );
                for eval in best_moves.iter().rev() {
                    println!("{eval}");
                }
            }
            SubCommand::Move { depth, seconds } => {
                let (duration, best_moves) = get_bot_move(
                    current_game_state,
                    player,
                    depth,
                    seconds.map(Duration::from_secs),
                    &mut session.cache,
                );
                let m = best_moves.into_iter().last().unwrap().best_move;
                println!("{} {:?}", m, duration);

                let next_game_state = execute_move_unchecked(current_game_state, &m);
                session.push(next_game_state, m);
            }
            SubCommand::Eval {
                move_to_evaluate,
                depth,
                seconds,
            } => {
                if let Some(move_str) = move_to_evaluate {
                    if let Some(m) = parse_player_move(&move_str) {
                        if is_move_legal(current_game_state, player, &m) {
                            let next_game_state = execute_move_unchecked(current_game_state, &m);
                            let (_, best_moves) = get_bot_move(
                                &next_game_state,
                                player,
                                depth,
                                seconds.map(Duration::from_secs),
                                &mut session.cache,
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
                        current_game_state,
                        player,
                        depth,
                        seconds.map(Duration::from_secs),
                        &mut session.cache,
                    );
                    println!(
                        "Best move evaluates to {}",
                        best_moves.last().unwrap().score
                    );
                }
            }
            SubCommand::ExportCache { file: path } => match std::fs::File::create(path) {
                Ok(file) => {
                    serde_json::ser::to_writer_pretty(file, &session.cache).unwrap();
                }
                Err(e) => {
                    println!("{:?}", e)
                }
            },
            SubCommand::ImportCache { file: path } => match std::fs::File::open(path) {
                Ok(file) => match serde_json::de::from_reader::<_, Cache>(file) {
                    Ok(cache) => session.cache = cache,
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

#[derive(clap_derive::Parser, Debug)]
pub struct AbeCommand {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum SubCommand {
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
    player: Player,
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
            player,
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
pub fn best_move_alpha_beta(
    game: &Game,
    player: Player,
    depth: usize,
    cache: &mut Cache,
) -> Vec<BoardEvaluation> {
    match alpha_beta(
        game,
        depth,
        WHITE_LOSES_BLACK_WINS,
        WHITE_WINS_BLACK_LOSES,
        player,
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
    player: Player,
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
    match player {
        Player::White => {
            let mut value = WHITE_LOSES_BLACK_WINS;
            for player_move in last
                .cloned()
                .into_iter()
                .chain(moves_ordered_by_heuristic_quality(game, player))
            {
                let child_game_state = execute_move_unchecked(&game, &player_move);
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
                    player.opponent(),
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
                .chain(moves_ordered_by_heuristic_quality(game, player))
            {
                let child_game_state = execute_move_unchecked(&game, &player_move);
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
                    player.opponent(),
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

fn moves_ordered_by_heuristic_quality(game: &Game, player: Player) -> Vec<PlayerMove> {
    let player_position = game.board.player_position(player);
    let opponent_position = game.board.player_position(player.opponent());

    let mut moves: Vec<PlayerMove> = all_move_piece_moves(player_position, opponent_position)
        .filter(move |move_piece| {
            is_move_piece_legal_with_player_at_position(
                &game.board,
                player,
                player_position,
                move_piece,
            )
        })
        .map(PlayerMove::MovePiece)
        .collect();
    if game.walls_left[player.as_index()] > 0 {
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
                    if room_for_wall_placement(&game.board, orientation, x, y) {
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
    player: Player,
    depth: Option<usize>,
    duration: Option<Duration>,
    cache: &mut Cache,
) -> (Duration, Vec<BoardEvaluation>) {
    let start_time = std::time::Instant::now();
    let best_moves = match (depth, duration) {
        (Some(depth), _) => best_move_alpha_beta(game, player, depth, cache),
        (_, duration) => {
            let duration = duration.unwrap_or(Duration::from_secs(3));
            best_move_alpha_beta_iterative_deepening(game, player, duration, cache)
        }
    };
    (start_time.elapsed(), best_moves)
}

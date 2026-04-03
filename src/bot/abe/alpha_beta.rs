use crate::{
    bot::abe::{heuristic::Heuristic, move_ordering::moves_ordered_by_heuristic_quality},
    data_model::{Game, PIECE_GRID_HEIGHT, Player, PlayerMove},
    game_logic::execute_move_unchecked,
    l_p_a_star::Pathfinding,
};
use std::{
    collections::HashMap,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

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
    pub transposition_table: Arc<Mutex<HashMap<u64, BoardEvaluation>>>,
}

fn hash_to_u64(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
pub enum AlphaBetaResult {
    Moves(BoardEvaluation),
    Stopped,
}
#[allow(clippy::too_many_arguments)]
pub fn alpha_beta<F>(
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

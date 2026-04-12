use crate::{
    bot::darwin::{
        Darwin,
        data_model::{EvaluatedGenes, EvaluatedPopulation, Genes, Population},
    },
    data_model::{Game, Player},
    game_logic::{execute_move_unchecked_inplace, player_has_won},
    generic_heuristic::GenericHeuristicWeights,
};
use rayon::prelude::*;

pub fn evaluate_population(population: Population) -> EvaluatedPopulation {
    let inner = population
        .0
        .iter()
        .map(|genes| evaluate_gene(genes.clone(), &population))
        .collect();
    EvaluatedPopulation(inner)
}

pub fn evaluate_gene(genes: Genes, population: &Population) -> EvaluatedGenes {
    let (wins, games) = population
        .0
        .par_iter()
        .filter(|opponent| &&genes != opponent)
        .map(|opponent| simulate_games(&genes, opponent))
        .reduce(
            || (0, 0),
            |(total_wins, total_games), (wins, games)| (total_wins + wins, total_games + games),
        );
    let win_rate = wins as f32 / games as f32;
    EvaluatedGenes { genes, win_rate }
}

fn simulate_games(
    genes: &GenericHeuristicWeights,
    opponent: &GenericHeuristicWeights,
) -> (usize, usize) {
    let number_of_games = 1;
    let wins = (0..number_of_games)
        .flat_map(|_| [Player::White, Player::Black].iter())
        .filter(|player| simulate_game(genes, opponent, player))
        .count();
    (wins, 2 * number_of_games)
}

fn simulate_game(
    genes: &GenericHeuristicWeights,
    opponent: &GenericHeuristicWeights,
    player: &Player,
) -> bool {
    let mut game = Game::new();
    let mut bot = Darwin {
        default_seconds: Some(1),
        default_weights: Default::default(),
        cache: Default::default(),
    };
    bot.default_weights[player.as_index()] = genes.clone();
    bot.default_weights[player.opponent().as_index()] = opponent.clone();
    let max_moves = 128;
    for _ in 0..max_moves {
        let m = bot.get_move_fixed_depth(&game);
        execute_move_unchecked_inplace(&mut game, &m);
        if let Some(winning_player) = player_has_won(&game.board) {
            if winning_player == *player {
                println!("Win:  {:?}", genes);
                println!("Loss: {:?}", opponent);
            } else {
                println!("Win:  {:?}", opponent);
                println!("Loss: {:?}", genes);
            }
            return winning_player == *player;
        }
    }
    println!("Tie:  {:?}", opponent);
    println!("Tie:  {:?}", genes);
    false
}

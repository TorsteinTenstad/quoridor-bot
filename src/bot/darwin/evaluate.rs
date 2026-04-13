use crate::{
    bot::darwin::{
        Darwin,
        data_model::{EvaluatedGenes, EvaluatedPopulation, Population},
    },
    data_model::{Game, Player},
    game_logic::{execute_move_unchecked_inplace, player_has_won},
    generic_heuristic::GenericHeuristicWeights,
};
use rand::{rng, seq::IteratorRandom};
use rayon::prelude::*;

pub fn evaluate_population(population: Population, depth: usize) -> EvaluatedPopulation {
    let n = population.0.len();
    let mut rng = rng();
    let matches_per_gene = 4;

    let matches: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| {
            (0..n)
                .filter(move |&j| j != i)
                .choose_multiple(&mut rng, matches_per_gene)
                .into_iter()
                .map(move |j| (i, j))
        })
        .collect();

    let results: Vec<(usize, usize, bool)> = matches
        .par_iter()
        .map(|&(i, j)| {
            let won = simulate_game(&population.0[i], &population.0[j], &Player::White, depth);
            (i, j, won)
        })
        .collect();

    let mut scores = vec![(0usize, 0usize); n];
    for (i, j, won) in results {
        scores[i].1 += 1;
        scores[j].1 += 1;
        if won {
            scores[i].0 += 1;
        } else {
            scores[j].0 += 1;
        }
    }

    let inner = population
        .0
        .into_iter()
        .zip(scores)
        .map(|(genes, (wins, games))| EvaluatedGenes {
            genes,
            win_rate: wins as f32 / games as f32,
        })
        .collect();

    EvaluatedPopulation(inner)
}

fn simulate_game(
    genes: &GenericHeuristicWeights,
    opponent: &GenericHeuristicWeights,
    player: &Player,
    depth: usize,
) -> bool {
    let mut game = Game::new();
    let mut bot = Darwin::default();
    bot.default_weights[player.as_index()] = genes.clone();
    bot.default_weights[player.opponent().as_index()] = opponent.clone();
    let max_moves = 128;
    for _ in 0..max_moves {
        let m = bot.get_move_fixed_depth(&game, depth);
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

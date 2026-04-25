use crate::bot::darwin::data_model::{EvaluatedGenes, EvaluatedPopulation, Genes, Population};
use rand::{Rng, RngExt};

pub fn evolve(population: EvaluatedPopulation) -> Population {
    let target_size = population.0.len();
    let mut rng = rand::rng();

    let mut evolved = Population::default();

    let best = population
        .0
        .iter()
        .max_by(|a, b| a.win_rate.total_cmp(&b.win_rate));
    if let Some(best) = best {
        evolved.0.push(best.genes.clone());
    }

    while evolved.0.len() < target_size {
        let parent_a = select(&population, &mut rng);
        let parent_b = select(&population, &mut rng);
        let child = crossover(parent_a, parent_b, &mut rng);
        evolved.0.push(mutate(&child, &mut rng));
        if evolved.0.len() < target_size {
            evolved.0.push(mutate(&parent_a.genes, &mut rng));
        }
        if evolved.0.len() < target_size {
            evolved.0.push(mutate(&parent_a.genes, &mut rng));
        }
    }

    evolved
}

fn select<'a>(population: &'a EvaluatedPopulation, rng: &mut impl Rng) -> &'a EvaluatedGenes {
    let a = &population.0[rng.random_range(0..population.0.len())];
    let b = &population.0[rng.random_range(0..population.0.len())];
    if a.win_rate >= b.win_rate { a } else { b }
}

fn crossover(a: &EvaluatedGenes, b: &EvaluatedGenes, rng: &mut impl Rng) -> Genes {
    let a_vec = a.genes.to_vec();
    let b_vec = b.genes.to_vec();

    let child_vec = a_vec
        .into_iter()
        .zip(b_vec)
        .map(|(a, b)| if rng.random::<bool>() { a } else { b })
        .collect::<Vec<_>>();

    Genes::from_slice(&child_vec).unwrap_or_default()
}

fn mutate(genes: &Genes, rng: &mut impl Rng) -> Genes {
    const MUTATION_RATE: f32 = 0.1;
    const MUTATION_STRENGTH: f32 = 0.1;

    let mutated = genes
        .to_vec()
        .into_iter()
        .map(|w| {
            if rng.random::<f32>() < MUTATION_RATE {
                w * (1.0 + rng.random_range(-MUTATION_STRENGTH..=MUTATION_STRENGTH))
            } else {
                w
            }
        })
        .collect::<Vec<_>>();

    Genes::from_slice(&mutated).unwrap()
}

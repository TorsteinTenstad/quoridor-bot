use std::{fs::File, path::PathBuf};

use lib::{
    bot::darwin::{
        data_model::{EvaluatedPopulation, Population},
        evaluate::evaluate_population,
        evolve::evolve,
    },
    generic_heuristic::GenericHeuristicWeights,
};
use rand::{Rng, RngExt, rng};

#[derive(clap_derive::Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap_derive::Subcommand, Debug)]
enum Command {
    Generate {
        #[clap(short, long)]
        n: usize,

        #[clap(short, long)]
        variance: f32,

        #[clap(short, long)]
        ouput: Option<PathBuf>,
    },
    Evaluate {
        #[clap()]
        input_population: PathBuf,

        #[clap(short, long)]
        output: Option<PathBuf>,

        #[clap(short, long)]
        depth: usize,
    },
    Evolve {
        #[clap()]
        input_population: PathBuf,

        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    Run {
        #[clap(short, long)]
        n: usize,

        #[clap(short, long)]
        depth: usize,

        #[clap(short, long)]
        generations: usize,

        #[clap(short, long, default_value = "out/")]
        output_dir: PathBuf,
    },
}

fn main() {
    let args = <Args as clap::Parser>::parse();

    match args.command {
        Command::Generate { n, variance, ouput } => {
            let mut rng = rng();
            let inner = (0..n)
                .map(|_| GenericHeuristicWeights {
                    distance: rng.random_range(-100.0..=100.0),
                    walls_left: rng.random_range(-100.0..=100.0),
                    opponent_distance_x_walls_left: rng.random_range(-100.0..=100.0),
                    manhattan_distance_x_wall_progress: rng.random_range(-100.0..=100.0),
                    walls_left_x_wall_progress: rng.random_range(-100.0..=100.0),
                })
                .collect();
            let population = Population(inner);
            let output_file = File::create(ouput.unwrap_or("generated.json".into())).unwrap();
            serde_json::to_writer_pretty(output_file, &population).unwrap();
        }
        Command::Evaluate {
            input_population,
            output,
            depth,
        } => {
            let output_path = output.unwrap_or({
                let input_name = input_population.file_name().unwrap();
                let output_name = format!("evaluated_{}", input_name.display());
                input_population.with_file_name(output_name)
            });
            let input_file = File::open(input_population).unwrap();
            let input_population = serde_json::from_reader::<_, Population>(input_file).unwrap();
            let evaluated = evaluate_population(input_population, depth);
            let output_file = File::create_new(output_path).unwrap();
            serde_json::to_writer_pretty(output_file, &evaluated).unwrap();
        }
        Command::Evolve {
            input_population,
            output,
        } => {
            let output_path = output.unwrap_or({
                let input_name = input_population.file_name().unwrap();
                let output_name = format!("evolved_{}", input_name.display());
                input_population.with_file_name(output_name)
            });
            let input_file = File::open(input_population).unwrap();
            let input_population =
                serde_json::from_reader::<_, EvaluatedPopulation>(input_file).unwrap();
            let evolved = evolve(input_population);
            let output_file = File::create_new(output_path).unwrap();
            serde_json::to_writer_pretty(output_file, &evolved).unwrap();
        }
        Command::Run {
            n,
            depth,
            generations,
            output_dir,
        } => {
            std::fs::create_dir_all(&output_dir).unwrap();

            let mut rng = rng();
            let inner = (0..n)
                .map(|_| GenericHeuristicWeights {
                    distance: rng.random_range(-100.0..=100.0),
                    walls_left: rng.random_range(-100.0..=100.0),
                    opponent_distance_x_walls_left: rng.random_range(-100.0..=100.0),
                    manhattan_distance_x_wall_progress: rng.random_range(-100.0..=100.0),
                    walls_left_x_wall_progress: rng.random_range(-100.0..=100.0),
                })
                .collect();
            let mut population = Population(inner);

            for generation in 0..generations {
                let evaluated = evaluate_population(population, depth);
                let output_path = output_dir.join(format!("generation_{generation:04}.json"));
                let output_file = File::create(output_path).unwrap();
                serde_json::to_writer_pretty(output_file, &evaluated).unwrap();
                population = evolve(evaluated);
            }

            println!("Done.");
        }
    }
}

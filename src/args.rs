use std::{path::PathBuf, string, time::Duration};

use crate::bot::BotType;

#[derive(clap_derive::Parser, Debug)]
pub struct Args {
    #[arg(short, long, group = "time_control")]
    pub depth: Option<usize>,

    #[arg(short, long, group = "time_control")]
    pub seconds: Option<u64>,

    #[arg(long, default_value_t = 0)]
    pub abe_background_threads: usize,

    #[arg(long)]
    pub abe_heuristic: Option<crate::bot::abe::heuristic::Heuristic>,

    #[arg(long)]
    pub dedi_heuristic_white: Option<crate::bot::dedi::heuristic::Heuristic>,

    #[arg(long)]
    pub dedi_heuristic_black: Option<crate::bot::dedi::heuristic::Heuristic>,

    #[arg(short = 'c', long, default_value_t = 1)]
    pub min_depth_for_caching: usize,

    #[clap(short, long, default_value_t = 0.0)]
    pub temperature: f32,

    #[clap(long)]
    pub w_nn_path: Option<PathBuf>,

    #[clap(long)]
    pub b_nn_path: Option<PathBuf>,

    #[clap(short = 'w', long)]
    pub player_white: Option<BotType>,

    #[clap(short = 'b', long)]
    pub player_black: Option<BotType>,

    #[clap(long, default_value_t = 1000)]
    pub window_size: usize,

    #[arg(long)]
    pub darwin_weights_white: Option<PathBuf>,

    #[arg(long)]
    pub darwin_weights_black: Option<PathBuf>,
}

pub const DEFAULT_DURATION: Duration = Duration::from_secs(5);

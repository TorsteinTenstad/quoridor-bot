use crate::bot::BotType;
use std::time::Duration;

#[derive(clap_derive::Parser, Debug)]
pub struct Args {
    #[arg(short, long, group = "time_control")]
    pub depth: Option<usize>,

    #[arg(short, long, group = "time_control")]
    pub seconds: Option<u64>,

    #[arg(short = 't', long, default_value_t = 0)]
    pub abe_background_threads: usize,

    #[arg(short, long)]
    pub heuristic: Option<crate::bot::abe::heuristic::Heuristic>,

    #[clap(short, long, default_value_t = 0.0)]
    pub temperature: f32,

    #[clap(short = 'w', long)]
    pub player_white: Option<BotType>,

    #[clap(short = 'b', long)]
    pub player_black: Option<BotType>,

    #[clap(long, default_value_t = 1000)]
    pub window_size: usize,
}

pub const DEFAULT_DURATION: Duration = Duration::from_secs(5);

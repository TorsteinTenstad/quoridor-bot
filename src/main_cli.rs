use std::collections::HashMap;

use clap::Parser;
use burn::backend::NdArray ;


use crate::commands::{Command, Session, execute_command, get_legal_command};
use crate::data_model::{Player};
use crate::player_type::{PlayerType};
use crate::nn_bot::{QuoridorNet};


pub mod all_moves;
pub mod nn_bot;
pub mod a_star;
pub mod bot;
pub mod commands;
pub mod data_model;
pub mod game_logic;
pub mod player_type;
pub mod render_board;
pub mod square_outline_iterator;

#[derive(clap_derive::Parser, Debug)]
struct Args {
    #[clap(short, long, default_value_t = 4)]
    depth: usize,

    #[clap(short, long, default_value_t = 0.0)]
    temperature: f32,

    #[clap(short='a', long, default_value_t = PlayerType::Human)]
    player_a: PlayerType,

    #[clap(short='b', long, default_value_t = PlayerType::Bot)]
    player_b: PlayerType,

    #[clap(short, long)]
    end_after_moves: Option<usize>,
}

fn main() {
    let args = Args::parse();

    let device = <NdArray as burn::prelude::Backend>::Device::default();

    let mut neural_networks: HashMap<Player, QuoridorNet> = HashMap::new();

    if args.player_a == PlayerType::NeuralNet
    {
        neural_networks.insert(Player::White, QuoridorNet::new());
    }
    if args.player_b == PlayerType::NeuralNet
    {
        neural_networks.insert(Player::Black, QuoridorNet::new());
    }

    let player_type = |p: Player| match p {
        Player::White => args.player_a,
        Player::Black => args.player_b,
    };
    let mut session = Session::new(neural_networks);

    for move_number in 0.. {
        let current_game_state = session.game_states.last().unwrap();
        let player = current_game_state.player;
        if let Some(end_after_moves) = args.end_after_moves
            && move_number >= end_after_moves
        {
            break;
        }
        println!("{}", render_board::render_board(&current_game_state.board));
        println!(
            "{} ({}) to move. Walls: White: {}, Black: {}",
            player.to_string(),
            player_type(player),
            current_game_state.walls_left[Player::White.as_index()],
            current_game_state.walls_left[Player::Black.as_index()]
        );

        let command = match player_type(player) {
            PlayerType::Human => get_legal_command(current_game_state, player),
            PlayerType::NeuralNet => {
                Command::AuxCommand(commands::AuxCommand::PlayNNMove {temperature: args.temperature})
            },
            PlayerType::Bot => Command::AuxCommand(commands::AuxCommand::PlayBotMove {
                depth: Some(args.depth),
                seconds: None,
            }),
        };
        execute_command(&mut session, command);
    }
}

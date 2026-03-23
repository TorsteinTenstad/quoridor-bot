use clap::Parser;
use ggez::{
    conf::WindowMode,
    event::{self, EventHandler},
    {Context, ContextBuilder, GameResult},
};
use lib::{
    commands::{self, Command, Session, execute_command, get_legal_command},
    data_model::{Game, Player},
    draw,
    nn_bot::QuoridorNet,
    player_type::PlayerType,
};
use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, channel},
};

#[derive(clap_derive::Parser, Debug)]
struct Args {
    #[arg(short, long, group = "time_control")]
    depth: Option<usize>,

    #[arg(short, long, group = "time_control")]
    seconds: Option<u64>,

    #[clap(short, long, default_value_t = 0.0)]
    temperature: f32,

    #[clap(short='a', long, default_value_t = PlayerType::Human)]
    player_a: PlayerType,

    #[clap(short='b', long, default_value_t = PlayerType::Bot)]
    player_b: PlayerType,

    #[clap(short, long)]
    end_after_moves: Option<usize>,

    #[clap(short, long, default_value_t = 1000)]
    window_size: usize,

    #[clap(long)]
    skip_initial_moves: bool,
}

fn main() {
    let args = Args::parse();

    let mut neural_networks: HashMap<Player, QuoridorNet> = HashMap::new();

    if args.player_a == PlayerType::NeuralNet {
        neural_networks.insert(Player::White, QuoridorNet::new());
    }
    if args.player_b == PlayerType::NeuralNet {
        neural_networks.insert(Player::Black, QuoridorNet::new());
    }

    let (ctx, event_loop) = ContextBuilder::new("quoridor-bot", "Torstein Tenstad")
        .window_mode(
            WindowMode::default()
                .resizable(true)
                .dimensions(args.window_size as f32, args.window_size as f32),
        )
        .build()
        .unwrap();
    let (tx, rx) = channel::<Game>();
    let gui_state = GuiState {
        rx,
        current_state: Game::new(),
    };

    std::thread::spawn(move || {
        let player_type = |p: Player| match p {
            Player::White => args.player_a,
            Player::Black => args.player_b,
        };
        let mut session = Session::new(neural_networks);
        loop {
            let current_game_state = session.game_states.last().unwrap();
            let player = current_game_state.player;
            println!(
                "{} ({}) to move. Walls: White: {}, Black: {}",
                player.to_string(),
                player_type(player),
                current_game_state.walls_left[Player::White.as_index()],
                current_game_state.walls_left[Player::Black.as_index()]
            );
            let command = match player_type(player) {
                PlayerType::Human => get_legal_command(current_game_state, player),
                PlayerType::NeuralNet => Command::AuxCommand(commands::AuxCommand::PlayNNMove {
                    temperature: args.temperature,
                }),
                PlayerType::Bot => Command::AuxCommand(commands::AuxCommand::PlayBotMove {
                    depth: args.depth,
                    seconds: args.seconds,
                }),
            };
            execute_command(&mut session, command);
            tx.send(session.game_states.last().unwrap().clone())
                .unwrap();
        }
    });

    event::run(ctx, event_loop, gui_state);
}

struct GuiState {
    rx: Receiver<Game>,
    current_state: Game,
}

impl EventHandler for GuiState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        if let Ok(game) = self.rx.try_recv() {
            self.current_state = game;
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        draw::draw(&self.current_state, ctx)
    }
}

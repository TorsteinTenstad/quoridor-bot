use clap::Parser;
use ggez::{
    conf::WindowMode,
    event::{self, EventHandler},
    {Context, ContextBuilder, GameResult},
};
use lib::{
    agent::{AgentType, Agents},
    commands::{Command, Session, execute_command, get_legal_command},
    data_model::{Game, Player},
    draw,
};
use std::{
    fmt::{Debug, Display},
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

    #[clap(short = 'w', long)]
    player_white: Option<AgentType>,

    #[clap(short = 'b', long)]
    player_black: Option<AgentType>,

    #[clap(short, long)]
    end_after_moves: Option<usize>,

    #[clap(short, long, default_value_t = 1000)]
    window_size: usize,

    #[clap(long)]
    skip_initial_moves: bool,
}

pub enum InputType {
    Manual,
    Automatic(AgentType),
}

impl From<Option<AgentType>> for InputType {
    fn from(value: Option<AgentType>) -> Self {
        match value {
            Some(agent_type) => InputType::Automatic(agent_type),
            None => InputType::Manual,
        }
    }
}

impl Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputType::Automatic(agent_type) => agent_type.fmt(f),
            InputType::Manual => write!(f, "manual"),
        }
    }
}

fn main() {
    let args = Args::parse();

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
        let input_type_white = InputType::from(args.player_white);
        let input_type_black = InputType::from(args.player_black);
        let mut agents_white = Agents::default();
        let mut agents_black = Agents::default();

        let mut session = Session::default();
        loop {
            let current_game_state = session.game_states.last().unwrap();
            let player = current_game_state.player;
            let (input_type, agents) = match player {
                Player::White => (&input_type_white, &mut agents_white),
                Player::Black => (&input_type_black, &mut agents_black),
            };
            println!(
                "{} ({}) to move. Walls: White: {}, Black: {}",
                player.to_string(),
                input_type,
                current_game_state.walls_left[Player::White.as_index()],
                current_game_state.walls_left[Player::Black.as_index()]
            );
            let command = match input_type {
                InputType::Manual => get_legal_command(current_game_state),
                InputType::Automatic(agent_type) => Command::PlayMove(
                    agents.get_move(session.game_states.last().unwrap(), agent_type),
                ),
            };
            execute_command(agents, &mut session, command);
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

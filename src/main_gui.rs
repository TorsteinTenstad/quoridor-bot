use clap::Parser;
use ggez::{
    conf::WindowMode,
    event::{self, EventHandler},
    {Context, ContextBuilder, GameResult},
};
use lib::{
    bot::{BotType, Bots},
    commands::{Command, execute_command, get_legal_command},
    data_model::{Game, Player},
    draw,
    session::Session,
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
    player_white: Option<BotType>,

    #[clap(short = 'b', long)]
    player_black: Option<BotType>,

    #[clap(short, long)]
    end_after_moves: Option<usize>,

    #[clap(short, long, default_value_t = 1000)]
    window_size: usize,

    #[clap(long)]
    skip_initial_moves: bool,
}

pub enum InputType {
    Manual,
    Automatic(BotType),
}

impl From<Option<BotType>> for InputType {
    fn from(value: Option<BotType>) -> Self {
        match value {
            Some(bot_type) => InputType::Automatic(bot_type),
            None => InputType::Manual,
        }
    }
}

impl Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputType::Automatic(bot_type) => bot_type.fmt(f),
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
        let mut bots_white = Bots::default();
        let mut bots_black = Bots::default();

        let mut session = Session::default();
        loop {
            let player = session.game.player;
            let (input_type, bots) = match player {
                Player::White => (&input_type_white, &mut bots_white),
                Player::Black => (&input_type_black, &mut bots_black),
            };
            println!(
                "{} ({}) to move. Walls: White: {}, Black: {}",
                player.to_string(),
                input_type,
                session.game.walls_left[Player::White.as_index()],
                session.game.walls_left[Player::Black.as_index()]
            );
            let command = match input_type {
                InputType::Manual => get_legal_command(&session.game),
                InputType::Automatic(bot_type) => {
                    Command::PlayMove(bots.get_move(&session.game, bot_type))
                }
            };
            execute_command(bots, &mut session, command);
            tx.send(session.game.clone()).unwrap();
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

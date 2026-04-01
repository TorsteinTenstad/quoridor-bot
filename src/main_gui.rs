use ggez::{
    conf::WindowMode,
    event::{self, EventHandler},
    {Context, ContextBuilder, GameResult},
};
use lib::{
    args::Args,
    bot::{BotType, Bots},
    commands::{Command, execute_command, get_legal_command},
    data_model::{Game, Player},
    draw,
    session::Session,
};
use std::{
    fmt::{Debug, Display},
    process::exit,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, channel},
    },
};

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
    let args = <Args as clap::Parser>::parse();

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

    let ctrl_c = Arc::new(AtomicBool::new(false));

    let c = ctrl_c.clone();
    ctrlc::set_handler(move || {
        if c.swap(true, Ordering::Relaxed) {
            exit(0);
        }
        println!("Aborting all automatic play. Ctrl+C again to abort");
    })
    .unwrap();

    std::thread::spawn(move || {
        let mut input_type_white = InputType::from(args.player_white);
        let mut input_type_black = InputType::from(args.player_black);
        let mut bots_white = Bots::default();
        let mut bots_black = Bots::default();
        bots_white.init(&args);
        bots_black.init(&args);

        let mut session: Session = Session::default();
        loop {
            if ctrl_c.load(Ordering::Relaxed) {
                input_type_white = InputType::Manual;
                input_type_black = InputType::Manual;
            }
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

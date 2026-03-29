use crate::{
    bot::{BotCommand, Bots},
    data_model::{Direction, Game, MovePiece, PlayerMove, WallOrientation, WallPosition},
    game_logic::is_move_legal,
    session::Session,
};
use clap::Parser;
use std::path::PathBuf;

pub enum Command {
    PlayMove(PlayerMove),
    AuxCommand(AuxCommand),
}

#[derive(clap_derive::Subcommand, Debug)]
pub enum AuxCommand {
    #[command(subcommand)]
    Bot(BotCommand),
    Reset,
    Undo {
        #[arg(default_value_t = 1)]
        moves: usize,
    },
    Export {
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    Import {
        #[arg(group = "source", default_value_t = String::default())]
        moves_string: String,

        #[arg(short, long, group = "source")]
        file: Option<PathBuf>,
    },
}
const AUX_COMMAND_NAME: &str = "";

#[derive(clap_derive::Parser, Debug)]
#[command(name = AUX_COMMAND_NAME)]
struct AuxCommandParserHelper {
    #[command(subcommand)]
    command: AuxCommand,
}

pub fn execute_command(bots: &mut Bots, session: &mut Session, command: Command) {
    match command {
        Command::PlayMove(m) => session.make_move(m),
        Command::AuxCommand(aux_command) => match aux_command {
            AuxCommand::Bot(bot_command) => bots.execute_bot_command(session, bot_command),
            AuxCommand::Reset => *session = Session::default(),
            AuxCommand::Undo { moves } => {
                for _ in 0..moves {
                    let Some(new_current) = session.game_history.pop() else {
                        break;
                    };
                    session.game = new_current;
                    session.move_history.pop();
                }
            }
            AuxCommand::Export { file } => {
                let exported = session
                    .move_history
                    .iter()
                    .map(|m| format!("{m};"))
                    .collect::<String>();
                match file {
                    None => {
                        println!("{exported}")
                    }
                    Some(path) => match std::fs::write(&path, exported) {
                        Err(e) => println!("{}", e),
                        Ok(()) => println!("Game saved to {}", path.display()),
                    },
                }
            }
            AuxCommand::Import { moves_string, file } => {
                let moves_string = match file {
                    None => moves_string,
                    Some(path) => match std::fs::read(path) {
                        Ok(vec) => String::from_utf8_lossy(&vec).into(),
                        Err(e) => {
                            println!("{:?}", e);
                            return;
                        }
                    },
                };
                if let Some(moves) = moves_string
                    .trim_matches(';')
                    .split(';')
                    .map(parse_player_move)
                    .collect::<Option<Vec<_>>>()
                {
                    *session = Session::default();
                    for m in moves {
                        session.make_move(m);
                    }
                }
            }
        },
    }
}

pub enum ParseCommandResult {
    Command(Command),
    HelpText(String),
    InvalidInput,
}

pub fn parse_command(input: &str) -> ParseCommandResult {
    match parse_player_move(input) {
        Some(player_move) => ParseCommandResult::Command(Command::PlayMove(player_move)),
        None => {
            match AuxCommandParserHelper::try_parse_from(
                std::iter::once(AUX_COMMAND_NAME).chain(input.split_whitespace()),
            ) {
                Ok(h) => ParseCommandResult::Command(Command::AuxCommand(h.command)),
                Err(e) => ParseCommandResult::HelpText(format!("{}", e)),
            }
        }
    }
}

pub fn get_legal_command(game: &Game) -> Command {
    use std::io::{self, Write};

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match parse_command(input) {
            ParseCommandResult::Command(Command::PlayMove(player_move))
                if !is_move_legal(game, &player_move) =>
            {
                println!("Invalid move.")
            }
            ParseCommandResult::Command(command) => break command,
            ParseCommandResult::HelpText(help_text) => println!("{}", help_text),
            ParseCommandResult::InvalidInput => println!("Invalid input format."),
        }
    }
}
pub fn parse_player_move(input: &str) -> Option<PlayerMove> {
    let mut chars = input.chars();

    let direction_from_char = |c: Option<char>| match c {
        Some('u') => Some(Direction::Up),
        Some('d') => Some(Direction::Down),
        Some('l') => Some(Direction::Left),
        Some('r') => Some(Direction::Right),
        _ => None,
    };

    match chars.next() {
        Some('m') => {
            let direction = direction_from_char(chars.next())?;
            let direction_on_collision = direction_from_char(chars.next()).unwrap_or(direction);
            Some(PlayerMove::MovePiece(MovePiece {
                direction,
                direction_on_collision,
            }))
        }
        Some('h') => match (chars.next(), chars.next()) {
            (Some(x), Some(y)) => {
                let x = x.to_digit(10)? as usize;
                let y = y.to_digit(10)? as usize;
                Some(PlayerMove::PlaceWall {
                    orientation: WallOrientation::Horizontal,
                    position: WallPosition { x, y },
                })
            }
            _ => None,
        },
        Some('v') => match (chars.next(), chars.next()) {
            (Some(x), Some(y)) => {
                let x = x.to_digit(10)? as usize;
                let y = y.to_digit(10)? as usize;
                Some(PlayerMove::PlaceWall {
                    orientation: WallOrientation::Vertical,
                    position: WallPosition { x, y },
                })
            }
            _ => None,
        },
        _ => None,
    }
}

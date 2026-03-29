use crate::{
    agent::{Agents, BotCommand},
    data_model::{Direction, Game, MovePiece, PlayerMove, WallOrientation, WallPosition},
    game_logic::{execute_move_unchecked, is_move_legal},
};
use clap::Parser;
use std::path::PathBuf;

#[derive(clap_derive::Subcommand, Debug)]
pub enum AuxCommand {
    Bot(BotSubCommand),
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
pub struct BotSubCommand {
    #[command(subcommand)]
    pub cmd: BotCommand,
}

#[derive(clap_derive::Parser, Debug)]
#[command(name = AUX_COMMAND_NAME)]
struct AuxCommandParserHelper {
    #[command(subcommand)]
    command: AuxCommand,
}

pub enum Command {
    PlayMove(PlayerMove),
    AuxCommand(AuxCommand),
}

pub struct Session {
    pub game_states: Vec<Game>,
    pub moves: Vec<PlayerMove>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            game_states: vec![Game::new()],
            moves: Default::default(),
        }
    }
}

impl Session {
    pub fn push(&mut self, game_state: Game, m: PlayerMove) {
        self.game_states.push(game_state);
        self.moves.push(m);
    }
}

pub fn execute_command(agents: &mut Agents, session: &mut Session, command: Command) {
    let current_game_state = session.game_states.last().unwrap();
    match command {
        Command::PlayMove(m) => {
            let next_game_state = execute_move_unchecked(current_game_state, &m);
            session.push(next_game_state, m);
        }
        Command::AuxCommand(aux_command) => match aux_command {
            AuxCommand::Bot(bot_command) => agents.execute_bot_command(session, bot_command.cmd),
            AuxCommand::Reset => *session = Session::default(),
            AuxCommand::Undo { moves } => {
                for _ in 0..moves {
                    if session.game_states.len() == 1 {
                        break;
                    }
                    session.game_states.pop();
                    session.moves.pop();
                }
            }
            AuxCommand::Export { file } => {
                let exported = session
                    .moves
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
                        let current_game_state = session.game_states.last().unwrap();
                        let next_game_state = execute_move_unchecked(current_game_state, &m);
                        session.push(next_game_state, m);
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

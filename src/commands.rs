use std::{collections::HashMap};

use clap::Parser;

use crate::{
    bot::{best_move_alpha_beta, best_move_alpha_beta_iterative_deepening},
    data_model::{Direction, Game, MovePiece, Player, PlayerMove, WallOrientation, WallPosition},
    game_logic::{execute_move_unchecked, is_move_legal},
    nn_bot::{self, QuoridorNet}
};

use std::{fmt::Display, time::Duration};

#[derive(clap_derive::Subcommand, Debug)]
pub enum AuxCommand {
    Reset,
    BotMove {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
    },
    PlayBotMove {
        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
    },
    PlayNNMove {
        #[arg(default_value_t = 0.0)]
        temperature: f32,
    },
    Undo {
        #[arg(default_value_t = 1)]
        moves: usize,
    },
    Eval {
        #[arg()]
        move_to_evaluate: Option<String>,

        #[arg(short, long, group = "time_control")]
        depth: Option<usize>,

        #[arg(short, long, group = "time_control")]
        seconds: Option<u64>,
    },
    Export,
    Import {
        #[arg()]
        moves_string: String,
    },
}
const AUX_COMMAND_NAME: &str = "";

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
    pub neural_networks: HashMap<Player, QuoridorNet>,
    pub moves: Vec<PlayerMove>,
}
impl Session {
    pub(crate) fn new(neural_networks: HashMap<Player, QuoridorNet>) -> Self {
        Self {
            game_states: vec![Game::new()],
            neural_networks: neural_networks,
            moves: Vec::new(),
        }
    }
}

pub fn execute_command(session: &mut Session, command: Command) {
    let current_game_state = session.game_states.last().unwrap();
    let player = current_game_state.player;
    match command {
        Command::PlayMove(player_move) => {
            let mut next_game_state = current_game_state.clone();
            execute_move_unchecked(&mut next_game_state, player, &player_move);
            session.game_states.push(next_game_state);
            session.moves.push(player_move);
        }
        Command::AuxCommand(aux_command) => match aux_command {
            AuxCommand::Reset => {*session = Session::new(HashMap::new())},
            AuxCommand::BotMove { depth, seconds } => {
                let bot_move = get_bot_move(
                    current_game_state,
                    player,
                    depth,
                    seconds.map(Duration::from_secs),
                );
                println!("{bot_move}");
            }
            AuxCommand::PlayBotMove { depth, seconds } => {
                let bot_move = get_bot_move(
                    current_game_state,
                    player,
                    depth,
                    seconds.map(Duration::from_secs),
                );
                println!("{bot_move}");
                let mut next_game_state = current_game_state.clone();
                execute_move_unchecked(&mut next_game_state, player, &bot_move.player_move);
                session.game_states.push(next_game_state);
                session.moves.push(bot_move.player_move);
            }
            AuxCommand::PlayNNMove {temperature} =>
            {
                let nn_move = nn_bot::get_move(&current_game_state, session.neural_networks.get(&player).unwrap(), player, temperature);
                
                let mut next_game_state = current_game_state.clone();
                execute_move_unchecked(&mut next_game_state, player, &nn_move);
                session.game_states.push(next_game_state);

            }
            AuxCommand::Undo { moves } => {
                for _ in 0..moves {
                    if session.game_states.len() == 1 {
                        break;
                    }
                    session.game_states.pop();
                    session.moves.pop();
                }
            }
            AuxCommand::Eval {
                move_to_evaluate,
                depth,
                seconds,
            } => {
                if let Some(move_str) = move_to_evaluate {
                    if let Some(player_move) = parse_player_move(&move_str) {
                        if is_move_legal(current_game_state, player, &player_move) {
                            let mut child_game_state = current_game_state.clone();
                            execute_move_unchecked(&mut child_game_state, player, &player_move);
                            let score = get_bot_move(
                                &child_game_state,
                                player,
                                depth,
                                seconds.map(Duration::from_secs),
                            );
                            println!("{}", score);
                        } else {
                            println!("Invalid move");
                        }
                    } else {
                        println!("Could not parse move: {}", move_str);
                    }
                } else {
                    let score = get_bot_move(
                        current_game_state,
                        player,
                        depth,
                        seconds.map(Duration::from_secs),
                    );
                    println!("Best move evaluates to {}", score);
                }
            }
            AuxCommand::Export => {
                for m in &session.moves {
                    print!("{m};");
                }
                println!();
            }
            AuxCommand::Import { moves_string } => {
                if let Some(moves) = moves_string
                    .trim_matches(';')
                    .split(';')
                    .map(parse_player_move)
                    .collect::<Option<Vec<_>>>()
                {
                    *session = Session::new(HashMap::new());
                    for player_move in moves {
                        let mut next_game_state = session.game_states.last().unwrap().clone();
                        let player = next_game_state.player;
                        execute_move_unchecked(&mut next_game_state, player, &player_move);
                        session.game_states.push(next_game_state);
                        session.moves.push(player_move);
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

pub fn get_legal_command(game: &Game, player: Player) -> Command {
    use std::io::{self, Write};

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match parse_command(input) {
            ParseCommandResult::Command(Command::PlayMove(player_move))
                if !is_move_legal(game, player, &player_move) =>
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

pub struct BotMove {
    player_move: PlayerMove,
    score: isize,
    depth: usize,
    planned_duration: Option<Duration>,
    actual_duration: Duration,
}

impl Display for BotMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.player_move)?;
        write!(f, " score:{}", self.score)?;
        write!(f, " depth:{}", self.depth)?;
        write!(f, " {:?}", self.actual_duration)?;
        if let Some(d) = self.planned_duration {
            write!(f, "({:?})", d)?;
        }
        Ok(())
    }
}

fn get_bot_move(
    game: &Game,
    player: Player,
    depth: Option<usize>,
    duration: Option<Duration>,
) -> BotMove {
    let start_time = std::time::Instant::now();
    let (score, best_move, depth, planned_duration) = match (depth, duration) {
        (Some(depth), _) => {
            let (score, best_move) = best_move_alpha_beta(game, player, depth);
            (score, best_move, depth, None)
        }
        (_, duration) => {
            let duration = duration.unwrap_or(Duration::from_secs(3));
            let (score, best_move, depth) =
                best_move_alpha_beta_iterative_deepening(game, player, duration);
            (score, best_move, depth, Some(duration))
        }
    };
    let elapsed = start_time.elapsed();
    BotMove {
        player_move: best_move.unwrap(),
        score,
        depth,
        planned_duration,
        actual_duration: elapsed,
    }
}

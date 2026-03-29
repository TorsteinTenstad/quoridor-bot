use crate::data_model::{Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player};

#[derive(Clone, Copy)]
enum Dir {
    Unreachable,
    Goal,
    Left,
    Right,
    Up,
    Down,
}

struct Board {
    board: [[Dir; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT],
}

impl From<&Game> for Board {
    fn from(game: &Game) -> Self {
        let mut board = [[Dir::Unreachable; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut visited = [[false; PIECE_GRID_WIDTH]; PIECE_GRID_HEIGHT];
        let mut queue = [(0 as u8, 0 as u8); PIECE_GRID_WIDTH * PIECE_GRID_HEIGHT];
        let mut queue_len = 0;

        //let pos = game.board.player_position(game.player);

        {
            let y = match game.player {
                Player::Black => 0,
                Player::White => PIECE_GRID_HEIGHT - 1,
            };
            for x in 0..PIECE_GRID_HEIGHT {
                board[y][x] = Dir::Goal;
                visited[y][x] = true;
                queue[queue_len] = (x as u8, y as u8);
                queue_len += 1;
            }
        }

        let i = 0;
        while i < queue_len {
            let (x, y) = queue[i];
        }

        todo!()
    }
}

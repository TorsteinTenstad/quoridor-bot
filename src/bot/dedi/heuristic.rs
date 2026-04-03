use crate::{
    bot::dedi::{
        minimax::INF,
        walls::{Board, Tile},
    },
    data_model::{Game, Player},
};

#[derive(Default, Debug, Clone, Copy, clap_derive::ValueEnum)]
pub enum Heuristic {
    #[default]
    Simple,
}

impl Heuristic {
    pub fn eval(&self, game: &Game, player: Player, board: &Board) -> isize {
        match &self {
            Self::Simple => simple(game, player, board),
        }
    }
}

fn simple(game: &Game, player: Player, board: &Board) -> isize {
    let pos = game.board.player_position(player);
    let dis = match board.tiles[pos.y][pos.x] {
        Tile::Valid(_, dis) => dis,
        Tile::Invalid => unreachable!(),
    };
    if dis == 0 {
        return INF;
    }

    let mut h: isize = 0;

    h -= dis as isize * 10;
    h += game.walls_left[player.as_index()] as isize * 15;

    h
}

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
    Tt,
}

impl Heuristic {
    pub fn eval(
        &self,
        game: &Game,
        player: Player,
        board: &Board,
        opponent_board: &Board,
    ) -> isize {
        match &self {
            Self::Simple => simple(game, player, board, opponent_board),
            Self::Tt => tt(game, player, board, opponent_board),
        }
    }
}

fn tt(game: &Game, player: Player, board: &Board, opponent_board: &Board) -> isize {
    let distance = {
        let pos = game.board.player_position(player);
        match board.tiles[pos.y][pos.x] {
            Tile::Valid(_, distance) => distance,
            Tile::Invalid => unreachable!(),
        }
    };
    let opponent_distance = {
        let pos = game.board.player_position(player.opponent());
        match opponent_board.tiles[pos.y][pos.x] {
            Tile::Valid(_, distance) => distance,
            Tile::Invalid => unreachable!(),
        }
    };
    if distance == 0 {
        return INF;
    }

    let mut h: isize = 0;

    h -= distance as isize * 10;
    h += game.walls_left[player.as_index()] as isize * (8 + opponent_distance as isize / 2);

    h
}

fn simple(game: &Game, player: Player, board: &Board, _opponent_board: &Board) -> isize {
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

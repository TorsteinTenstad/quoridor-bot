use crate::{
    agent::Agent,
    all_moves::ALL_MOVES,
    data_model::{Game, PlayerMove},
};

pub struct Random;

impl Agent for Random {
    fn get_move(&mut self, _game: &Game) -> PlayerMove {
        // TODO: random valid move
        ALL_MOVES[0].clone()
    }
}

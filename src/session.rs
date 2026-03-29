use crate::{
    data_model::{Game, PlayerMove},
    game_logic::execute_move_unchecked,
};

pub struct Session {
    pub game: Game,
    pub game_history: Vec<Game>,
    pub move_history: Vec<PlayerMove>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            game: Game::new(),
            game_history: Default::default(),
            move_history: Default::default(),
        }
    }
}

impl Session {
    pub fn push(&mut self, game_state: Game, m: PlayerMove) {
        self.move_history.push(m);
        self.game_history.push(self.game.clone());
        self.game = game_state;
    }
    pub fn make_move(&mut self, m: PlayerMove) {
        self.push(execute_move_unchecked(&self.game, &m), m);
    }
}

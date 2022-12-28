pub mod game;
pub mod players;
pub mod struggle;

#[derive(Debug, Default)]
pub struct GameStats {
    pub move_distribution: [[u16; 4]; 2],
    pub turns: u16,
}

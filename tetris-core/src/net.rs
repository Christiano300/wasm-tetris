use serde::{Deserialize, Serialize};

use crate::tetris::{Board, GameConfig, GameSettings};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Start(GameConfig),
    LineSend(u8),
    GameState(Box<Board>),
    Gameover,
    Disconnect,
}

#[derive(Serialize, Deserialize)]
pub struct HighscoreReq {
    pub auth: String,
    pub name: String,
    pub settings: GameSettings,
    pub was_multiplayer: bool,
    pub score: u32,
}

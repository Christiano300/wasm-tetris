use serde::{Deserialize, Serialize};

use crate::tetris::{Board, GameConfig};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Start(GameConfig),
    LineSend(u8),
    GameState(Box<Board>),
    Gameover,
    Disconnect,
}

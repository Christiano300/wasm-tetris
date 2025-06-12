use serde::{Deserialize, Serialize};

use crate::tetris::{Board, GameSettings};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Start(GameSettings),
    LineSend(u8),
    GameState(Box<Board>),
    Gameover,
    Disconnect,
}

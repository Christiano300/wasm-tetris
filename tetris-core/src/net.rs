use serde::{Deserialize, Serialize};

use crate::tetris::GameSettings;

#[derive(Serialize, Deserialize)]
pub enum Message {
    Start(GameSettings),
    LineSend(u8),
    Gameover,
    Disconnect,
}

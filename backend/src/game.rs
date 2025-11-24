use actix_web::web::Bytes;
use actix_ws::Session;
use log::warn;
use serde::Serialize;
use std::fmt::Debug;

use crate::proto::TetrisSocket;
use tetris_core::{
    net::Message,
    tetris::{GameConfig, GameSettings},
};

pub enum Game {
    /// One player is waiting, this is the lobby
    Waiting {
        p1: Session,
        id: String,
        settings: GameSettings,
    },
    /// Second player joins the game, waiting for both websockets to connect
    Ready {
        p1: Option<Session>,
        p1_id: String,
        p2: Option<Session>,
        p2_id: String,
        id: String,
        settings: GameSettings,
    },
    /// Both clients have connected, as soon as this is reached, the start command is sent
    Running {
        p1: TetrisSocket,
        p2: TetrisSocket,
        id: String,
        config: GameConfig,
    },
}

impl Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Game::Waiting { id, .. } => f.debug_struct("Waiting").field("id", id).finish(),
            Game::Ready { id, .. } => f.debug_struct("Ready").field("id", id).finish(),
            Game::Running { id, .. } => f.debug_struct("Running").field("id", id).finish(),
        }
    }
}

impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.get_settings().serialize(serializer)
    }
}

impl Game {
    pub async fn client_timeout(&mut self) {
        if let Self::Ready { p1, p2, .. } = self {
            if let Some(session) = p1 {
                let _ = session.text("cancel timeout").await;
            }
            if let Some(session) = p2 {
                let _ = session.text("cancel timeout").await;
            }
        } else if let Self::Running { p1, p2, .. } = self {
            p1.clone().canceled("timeout").await;
            p2.clone().canceled("timeout").await;
        }
    }

    pub fn get_id(&self) -> &String {
        match self {
            Game::Waiting { id, .. } | Game::Ready { id, .. } | Game::Running { id, .. } => id,
        }
    }

    pub fn get_settings(&self) -> &GameSettings {
        match &self {
            Game::Waiting { settings, .. } | Game::Ready { settings, .. } => settings,
            Game::Running { config, .. } => &config.settings,
        }
    }

    pub async fn recv(&mut self, msg: &Bytes, player_id: &str) {
        let (_this, other) = self.get_sockets(player_id);
        let Ok(message) = serde_cbor::from_slice(msg) else {
            warn!("Invalid message received from Websocket");
            return;
        };
        match message {
            Message::LineSend(lines) => other.line_send(lines).await,

            // only meant for S2C
            Message::Start { .. } => {}

            // relay everything else directly
            msg => {
                let _ = other.send(&msg).await;
            }
        }
    }

    pub async fn start(&mut self) {
        if let Game::Running { p1, p2, config, .. } = self {
            let _ = p1.send(&Message::Start(*config)).await;
            let _ = p2.send(&Message::Start(*config)).await;
        }
    }

    fn get_sockets(&mut self, id: &str) -> (&mut TetrisSocket, &mut TetrisSocket) {
        if let Game::Running { p1, p2, .. } = self {
            if p1.id == id {
                (p1, p2)
            } else {
                (p2, p1)
            }
        } else {
            panic!("Tried to get players of non-running game")
        }
    }
}

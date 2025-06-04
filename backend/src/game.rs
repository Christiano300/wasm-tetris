use actix_web::web::Bytes;
use actix_ws::Session;
use log::warn;
use std::fmt::Debug;

use crate::proto::TetrisSocket;
use tetris_core::net::Message;

pub enum Game {
    Waiting {
        p1: Session,
        id: String,
    },
    Ready {
        p1: Option<Session>,
        p1_id: String,
        p2: Option<Session>,
        p2_id: String,
        id: String,
    },
    Running {
        p1: TetrisSocket,
        p2: TetrisSocket,
        id: String,
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

    pub async fn recv(&mut self, msg: &Bytes, player_id: &str) {
        let (_this, other) = self.get_sockets(player_id);
        let Ok(message) = serde_cbor::from_slice(msg) else {
            warn!("Invalid message received from Websocket");
            return;
        };
        match message {
            Message::LineSend(lines) => other.line_send(lines).await,
            Message::Start => {}
        }
    }

    pub async fn start(&mut self) {
        if let Game::Running { p1, p2, .. } = self {
            let _ = p1.send(&Message::Start).await;
            let _ = p2.send(&Message::Start).await;
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

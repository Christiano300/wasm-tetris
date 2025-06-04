use actix_ws::{CloseCode, CloseReason, Closed, Session};
use log::info;
use serde::Serialize;
use serde_cbor::Serializer;
use tetris_core::net::Message;

#[derive(Clone)]
pub struct TetrisSocket {
    session: Session,
    pub id: String,
}

impl TetrisSocket {
    pub const fn new(session: Session, id: String) -> Self {
        Self { session, id }
    }

    pub async fn close(self, reason: Option<CloseReason>) -> Result<(), Closed> {
        self.session.close(reason).await
    }

    pub async fn canceled(mut self, reason: &str) {
        let _ = self.session.text(format!("cancel {reason}")).await;
        let _ = self.close(Some((CloseCode::Away, reason).into())).await;
    }

    pub async fn send(&mut self, msg: &Message) -> Result<(), Closed> {
        let mut data = Vec::new();
        let mut serializer = Serializer::new(&mut data).packed_format();
        let _ = msg.serialize(&mut serializer);
        self.session.binary(data).await
    }

    pub async fn line_send(&mut self, lines: u8) {
        info!("Line send with {lines} lines");
        let _ = self.send(&Message::LineSend(lines)).await;
    }
}

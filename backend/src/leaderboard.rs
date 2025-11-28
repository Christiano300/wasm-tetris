use std::{cmp::Ordering, collections::BTreeSet};

use actix_web::{HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tetris_core::{net::HighscoreReq, tetris::GameSettings};
use tokio::sync::Mutex;

use crate::{STORE_TOKEN, Store, auth::try_auth};

#[derive(Debug)]
pub struct Leaderboard {
    board: Mutex<BTreeSet<Entry>>,
}

impl Leaderboard {
    pub async fn serialize(&self) -> serde_json::Result<String> {
        let board = self.board.lock().await;
        serde_json::to_string(&*board)
    }

    pub fn deserialize(s: &str) -> serde_json::Result<Leaderboard> {
        Ok(Self {
            board: Mutex::new(serde_json::from_str(s)?),
        })
    }

    /// Attempts to add an entry to the leaderboard. Returns if the entry was actually added.
    /// Otherwise, a 400 should be sent
    pub async fn add_entry(&self, req: HighscoreReq, store: &Store) -> HttpResponse {
        if !try_auth(&req) {
            return HttpResponse::Unauthorized().finish();
        }

        if req.settings.easy {
            return HttpResponse::BadRequest()
                .body("Games in easy mode are not eligable for a highscore");
        }

        if req.name.len() > 50 {
            return HttpResponse::BadRequest().body("Name is too long");
        }

        let mut board = self.board.lock().await;
        board.insert(Entry {
            score: req.score,
            name: req.name,
            was_multiplayer: req.was_multiplayer,
            was_random: req.settings.random,
            mode: Mode::from_settings(req.settings),
        });
        drop(board);
        store
            .set(
                STORE_TOKEN,
                self.serialize()
                    .await
                    .expect("could not serialize leaderboard"),
            )
            .expect("falied to set store item");
        HttpResponse::Ok().finish()
    }

    pub async fn get_leaderboard(&self) -> impl Responder + use<> {
        let board = self.board.lock().await;

        HttpResponse::Ok().json(board.iter().rev().collect::<Vec<_>>())
    }

    pub fn new() -> Self {
        Self {
            board: Mutex::new(BTreeSet::new()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Mode {
    Normal,
    Jupiter,
    Nes,
    Crazy,
}

impl Mode {
    pub const fn from_settings(settings: GameSettings) -> Self {
        match (settings.jupiter, settings.nes) {
            (true, true) => Mode::Crazy,
            (true, false) => Mode::Jupiter,
            (false, true) => Mode::Nes,
            (false, false) => Mode::Normal,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Entry {
    score: u32,
    name: String,
    was_multiplayer: bool,
    was_random: bool,
    mode: Mode,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .cmp(&other.score)
            .then(self.was_multiplayer.cmp(&other.was_multiplayer))
            .then(self.was_random.cmp(&other.was_random))
            .then(self.name.cmp(&other.name))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

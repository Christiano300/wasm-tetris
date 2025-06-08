use std::{sync::Arc, time::Duration};

use actix::clock::{interval, Instant};
use actix_web::{rt::pin, web};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use log::info;
use tokio::{select, sync::Mutex};

use crate::{game::Game, Games};

static HB_INTERVAL: Duration = Duration::from_secs(5);
static TIMEOUT: Duration = Duration::from_secs(15);

pub async fn waiting_cancel(session: Session, state: web::Data<Games>, id: &String) {
    let mut lock = state.games.lock().await;
    let mut remove = false;
    if let Some(game) = lock.get(id) {
        let game = game.lock().await;
        remove = matches!(*game, Game::Waiting { .. });
    }
    if remove {
        lock.remove(id);
        drop(lock);
        state.updated().await;
    }
    let _ = session.close(None).await;
}

pub async fn ws_waiting(
    state: web::Data<Games>,
    id: String,
    mut session: Session,
    mut stream: AggregatedMessageStream,
) {
    let _ = session.text(format!("lobby {id}")).await;
    info!("Waiting Websocket started");
    let mut last_msg = Instant::now();
    let mut interval = interval(HB_INTERVAL);
    loop {
        pin!(let tick = interval.tick(););

        select! {
            _ = tick => {
                if Instant::now().duration_since(last_msg) > TIMEOUT {
                    info!("Websocket timed out");
                    waiting_cancel(session, state, &id).await;
                    break;
                }
                let _ = session.ping(b"").await;
            },

            msg = stream.recv() => {
                if let Some(Ok(msg)) = msg {
                    match msg {
                        AggregatedMessage::Ping(bytes) => {
                            info!("Pong");
                            let _ = session.pong(&bytes).await;
                        },
                        AggregatedMessage::Close(_) => {
                            info!("Session closed by client");
                            waiting_cancel(session, state, &id).await;
                            break;
                        },
                        _ => {}
                    }
                    last_msg = Instant::now();
                } else {
                    info!("Recv not Ok");
                    waiting_cancel(session, state, &id).await;
                    break;
                }
            }
        }
    }
}

async fn running_cancel(state: web::Data<Games>, game: Arc<Mutex<Game>>) {
    let mut lock = game.lock().await;
    state.games.lock().await.remove(lock.get_id());
    lock.client_timeout().await;
}

pub async fn ws_running(
    state: web::Data<Games>,
    game: Arc<Mutex<Game>>,
    player_id: String,
    mut session: Session,
    mut stream: AggregatedMessageStream,
) {
    info!("Running Websocket started");
    let mut last_msg = Instant::now();
    let mut interval = interval(HB_INTERVAL);
    loop {
        pin!(let tick = interval.tick(););

        select! {
            _ = tick => {
                if Instant::now().duration_since(last_msg) > TIMEOUT {
                    info!("Websocket timed out");
                    running_cancel(state, game).await;
                    break;
                }
                let _ = session.ping(b"").await;
            },

            msg = stream.recv() => {
                if let Some(Ok(msg)) = msg {
                    match msg {
                        AggregatedMessage::Ping(bytes) => {
                            info!("Pong");
                            let _ = session.pong(&bytes).await;
                        },
                        AggregatedMessage::Close(_) => {
                            info!("Session closed by client");
                            running_cancel(state, game).await;
                            break;
                        }
                        AggregatedMessage::Binary(bytes) => {
                            game.lock().await.recv(&bytes, &player_id).await;
                        }
                        _ => {}
                    }
                    last_msg = Instant::now();
                } else {
                    info!("Recv not Ok");
                    running_cancel(state, game).await;
                    break;
                }
            }
        }
    }
}

use actix_cors::Cors;
use actix_web::{
    App, Error, HttpRequest, HttpResponse, HttpServer, Responder, get, middleware::Compress, post,
    rt, web,
};
use broadcast::Broadcaster;
use game::Game;
use leaderboard::Leaderboard;
use log::info;
use persistent_kv::{Config, PersistentKeyValueStore};
use proto::TetrisSocket;
use rand::{Rng, distr::Alphanumeric};
use replace_with::replace_with_or_abort;
use std::{collections::HashMap, sync::Arc};
use tetris_core::{
    net::HighscoreReq,
    tetris::{GameConfig, GameSettings, RandomSeed},
};
use tokio::sync::Mutex;
use ws::{ws_running, ws_waiting};

mod auth;
mod broadcast;
mod game;
mod leaderboard;
mod proto;
mod ws;

type Store = PersistentKeyValueStore<String, String>;

#[cfg(windows)]
static SAVE_PATH: &str = "tetris_leaderboard.store";

#[cfg(not(windows))]
static SAVE_PATH: &str = "/home/christian/tetris_leaderboard.store";

static STORE_TOKEN: &str = "b";

struct Games {
    games: Mutex<HashMap<String, Arc<Mutex<Game>>>>,
    broadcaster: Arc<Broadcaster>,
}

impl Games {
    pub async fn serialize(&self) -> String {
        let map_guard = self.games.lock().await;

        // Clone Arc pointers so we can release map lock early
        let entries: Vec<_> = map_guard
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect();
        drop(map_guard); // release map lock early!

        // Collect serialized objects
        let mut result = HashMap::new();
        for (key, inner_mutex) in entries {
            let obj_guard = inner_mutex.lock().await;
            if matches!(*obj_guard, Game::Waiting { .. }) {
                result.insert(key, serde_json::to_value(&*obj_guard).unwrap()); // assuming Object: Clone + Serialize
            }
        }

        serde_json::to_string(&result).unwrap()
    }

    pub async fn updated(&self) {
        self.broadcaster.broadcast(&self.serialize().await).await;
    }

    pub async fn new_listener(&self) -> impl Responder + use<> {
        self.broadcaster.new_client(&self.serialize().await).await
    }
}

fn get_id() -> String {
    let mut random = rand::rng();
    (0..10)
        .map(|_| random.sample(Alphanumeric) as char)
        .collect()
}

fn game_config(settings: GameSettings) -> GameConfig {
    let mut rng = rand::rng();
    let mut buffer = RandomSeed::default();
    rng.fill(&mut buffer);
    GameConfig::with_seed(settings, buffer)
}

#[get("/create-game")]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<Games>,
    settings: web::Query<GameSettings>,
) -> Result<impl Responder, Error> {
    info!("WS Request {req:?}");
    let (response, session, stream) = actix_ws::handle(&req, stream)?;

    let id = get_id();
    let game = Arc::new(Mutex::new(Game::Waiting {
        p1: session.clone(),
        id: id.clone(),
        settings: *settings,
    }));
    let mut lock = state.games.lock().await;
    lock.insert(id.clone(), game.clone());
    drop(lock);

    state.updated().await;

    let stream = stream.aggregate_continuations();
    rt::spawn(ws_waiting(state.clone(), id, session, stream));

    Ok(response)
}

#[get("/join-game/{id}")]
async fn join(state: web::Data<Games>, path: web::Path<String>) -> impl Responder {
    let game_id = path.into_inner();

    let mut lock = state.games.lock().await;
    let Some(game_arc) = lock.get_mut(&game_id) else {
        return HttpResponse::NotFound().finish();
    };
    let game_arc = Arc::clone(game_arc);
    drop(lock);
    let mut game = game_arc.lock().await;
    let p1 = get_id();
    let p2 = get_id();
    let settings;
    if let Game::Waiting {
        p1: session,
        id,
        settings: s,
    } = &mut *game
    {
        let _ = session.text(format!("ready {id}/{p1}")).await;
        settings = s;
    } else {
        return HttpResponse::Conflict().finish();
    }
    *game = Game::Ready {
        p1: None,
        p1_id: p1,
        p2: None,
        p2_id: p2.clone(),
        id: game_id.clone(),
        settings: *settings,
    };
    drop(game);
    state.updated().await;

    HttpResponse::Ok().json(format!("{game_id}/{p2}"))
}

#[get("/connect/{game}/{player}")]
async fn connect(
    request: HttpRequest,
    state: web::Data<Games>,
    stream: web::Payload,
    path: web::Path<(String, String)>,
) -> Result<impl Responder, Error> {
    info!("Connect attempt");
    let (game_id, player_id) = path.into_inner();
    let mut lock = state.games.lock().await;
    let Some(game_arc) = lock.get_mut(&game_id) else {
        info!("Nonexistent game: {game_id}");
        return Ok(HttpResponse::NotFound().finish());
    };
    let game_arc = Arc::clone(game_arc);
    drop(lock);

    let mut game = game_arc.lock().await;
    let is_p1;
    if let Game::Ready {
        p1,
        p1_id,
        p2,
        p2_id,
        ..
    } = &*game
    {
        if p1.is_none() && *p1_id == player_id {
            is_p1 = true;
        } else if p2.is_none() && *p2_id == player_id {
            is_p1 = false;
        } else {
            info!("Cannot join, no slot free");
            return Ok(HttpResponse::Conflict().finish());
        }
    } else {
        info!("Game not ready");
        return Ok(HttpResponse::Conflict().finish());
    }

    let (res, session, stream) = actix_ws::handle(&request, stream)?;

    replace_with_or_abort(&mut *game, |game| {
        let Game::Ready {
            p1,
            p1_id,
            p2,
            p2_id,
            id,
            settings,
        } = game
        else {
            unreachable!()
        };
        if is_p1 {
            match p2 {
                Some(existing) => Game::Running {
                    p1: TetrisSocket::new(session.clone(), p1_id),
                    p2: TetrisSocket::new(existing, p2_id),
                    id,
                    config: game_config(settings),
                },
                None => Game::Ready {
                    p1: Some(session.clone()),
                    p1_id,
                    p2,
                    p2_id,
                    id,
                    settings,
                },
            }
        } else {
            match p1 {
                Some(existing) => Game::Running {
                    p1: TetrisSocket::new(existing, p1_id),
                    p2: TetrisSocket::new(session.clone(), p2_id),
                    id,
                    config: game_config(settings),
                },
                None => Game::Ready {
                    p1,
                    p1_id,
                    p2: Some(session.clone()),
                    p2_id,
                    id,
                    settings,
                },
            }
        }
    });

    if matches!(*game, Game::Running { .. }) {
        game.start().await;
        info!("Starting game {}", game.get_id());
    }

    let stream = stream.aggregate_continuations();
    rt::spawn(ws_running(
        state.clone(),
        game_arc.clone(),
        player_id,
        session,
        stream,
    ));

    Ok(res)
}

#[get("/games")]
async fn all_games(state: web::Data<Games>) -> impl Responder {
    state.new_listener().await
}

#[get("/leaderboard")]
async fn board_index(state: web::Data<Leaderboard>) -> impl Responder {
    state.get_leaderboard().await
}

#[post("/highscore")]
async fn highscore(
    req: web::Json<HighscoreReq>,
    state: web::Data<Leaderboard>,
    store: web::Data<Store>,
) -> impl Responder {
    state.add_entry(req.into_inner(), &store).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let store = Store::new(SAVE_PATH, Config::default()).expect("Could not create store");
    let state = web::Data::new(match store.get(STORE_TOKEN) {
        Some(l) => Leaderboard::deserialize(&l).expect("failed to deserialize leaderboard"),
        None => Leaderboard::new(),
    });
    let games: web::Data<Games> = web::Data::new(Games {
        games: Mutex::new(HashMap::new()),
        broadcaster: Broadcaster::create(),
    });
    let store = web::Data::new(store);
    info!("{state:?}");
    info!("Server starting");
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .wrap(Compress::default())
            .service(board_index)
            .service(highscore)
            .service(ws_index)
            .service(join)
            .service(connect)
            .service(all_games)
            .app_data(state.clone())
            .app_data(games.clone())
            .app_data(store.clone())
    })
    // .bind(("localhost", 4444))?
    // .bind(("172.21.49.178", 4444))?
    .bind(("0.0.0.0", 4444))?
    .run()
    .await
}

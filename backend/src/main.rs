use actix_cors::Cors;
use actix_web::{
    get, middleware::Compress, post, rt, web, App, Error, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use broadcast::Broadcaster;
use game::Game;
use log::info;
use persistent_kv::{Config, PersistentKeyValueStore};
use proto::TetrisSocket;
use rand::{distr::Alphanumeric, Rng};
use replace_with::replace_with_or_abort;
use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
use tetris_core::tetris::{GameConfig, GameSettings, RandomSeed};
use tokio::sync::Mutex;
use ws::{ws_running, ws_waiting};

mod auth;
mod broadcast;
mod game;
mod proto;
mod ws;

type Store = PersistentKeyValueStore<String, String>;

#[cfg(windows)]
static SAVE_PATH: &str = "tetris_leaderboard.store";

#[cfg(not(windows))]
static SAVE_PATH: &str = "/home/christian/tetris_leaderboard.store";

static STORE_TOKEN: &str = "b";

#[derive(Deserialize)]
struct HighscoreReq {
    auth: String,
    name: String,
    score: u32,
}

#[derive(Debug)]
struct Leaderboard {
    board: Mutex<BTreeSet<(u32, String)>>,
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
}

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

    pub async fn new_listener(&self) -> impl Responder {
        self.broadcaster.new_client(&self.serialize().await).await
    }
}

fn get_id() -> String {
    let mut random = rand::rng();
    (0..10)
        .map(|_| random.sample(Alphanumeric) as char)
        .collect()
}

fn try_auth(req: &HighscoreReq) -> bool {
    if auth::gen_auth_token(req) == req.auth {
        return true;
    }
    false
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
    info!("Got here 0");
    let (response, session, stream) = actix_ws::handle(&req, stream)?;

    info!("Got here 1");

    let id = get_id();
    let game = Arc::new(Mutex::new(Game::Waiting {
        p1: session.clone(),
        id: id.clone(),
        settings: *settings,
    }));
    let mut lock = state.games.lock().await;
    info!("Got here 3");
    lock.insert(id.clone(), game.clone());
    drop(lock);
    info!("Got here 4");
    state.updated().await;
    info!("Got here 5");

    let stream = stream.aggregate_continuations();
    info!("Got here 6");
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
        p1: ref mut session,
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
async fn board_index(state: web::Data<Leaderboard>) -> Result<impl Responder, Error> {
    info!("Highscore request");
    let board = state.board.lock().await;
    let map = board
        .iter()
        .rev()
        .enumerate()
        .map(|(i, (score, name))| format!("{}. {name}: {score}\n", i + 1));
    Ok(HttpResponse::Ok().body(map.collect::<String>()))
}

#[post("/highscore")]
async fn highscore(
    info: web::Json<HighscoreReq>,
    state: web::Data<Leaderboard>,
    store: web::Data<Store>,
) -> impl Responder {
    if !try_auth(&info) {
        return HttpResponse::Unauthorized().finish();
    }
    let mut board = state.board.lock().await;
    board.insert((info.score, info.name.clone()));
    drop(board);
    let leaderboard = &state;
    store
        .set(
            STORE_TOKEN,
            leaderboard
                .serialize()
                .await
                .expect("could not serialize leaderboard"),
        )
        .expect("falied to set store item");

    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let store = Store::new(SAVE_PATH, Config::default()).expect("Could not create store");
    let state = web::Data::new(match store.get(STORE_TOKEN) {
        Some(l) => Leaderboard::deserialize(&l).expect("failed to deserialize leaderboard"),
        None => Leaderboard {
            board: Mutex::new(BTreeSet::new()),
        },
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

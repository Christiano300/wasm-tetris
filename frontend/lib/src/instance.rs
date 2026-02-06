#![allow(clippy::future_not_send)]

use async_io_stream::IoStream;
use futures_codec::Framed;
use futures_util::{
    SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
#[cfg(feature = "export")]
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};
#[cfg(feature = "export")]
use tetris_core::tetris::Action;
use wasm_bindgen_futures::spawn_local;

use crate::{
    codec::CborCodec,
    draw::DrawingContext,
    input::{FrameInputs, InputManager},
    tetris_confirm, tetris_prompt,
};
use js_sys::Function;
use tetris_core::{
    net::{HighscoreReq, Message},
    tetris::{Board, Event, Game, GameConfig, GameSettings, Mino, Phase, Tetrimino},
};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, Headers, RequestInit, window};
use ws_stream_wasm::{WsMeta, WsStreamIo};

type MessageCodec = CborCodec<Message, Message>;
type TetrisFrames = Framed<IoStream<WsStreamIo, Vec<u8>>, MessageCodec>;

type TetrisSession = SplitSink<TetrisFrames, Message>;
type TetrisStream = SplitStream<TetrisFrames>;

const SHARE_COOLDOWN: u8 = 15;
const EMPTY_BOARD: Board = Board::new();

#[cfg(feature = "export")]
#[derive(Serialize)]
struct ExportFrame {
    pub board: Board,
    pub piece: Tetrimino,
    pub ghost: Option<Tetrimino>,
    pub hold: Option<Tetrimino>,
    pub next_queue: Vec<Mino>,
    pub score: u32,
    pub level: u8,
    pub inputs: Vec<Action>,
}

#[wasm_bindgen]
pub struct Instance {
    auth_func: Function,
    backend_url: String,
    context: Rc<CanvasRenderingContext2d>,
    drawing_context: DrawingContext,
    input_manager: InputManager,
    game: Rc<RefCell<Option<Game>>>,
    session: Rc<RefCell<Option<TetrisSession>>>,
    opponent_board: Rc<RefCell<Option<Board>>>,
    messages: Rc<RefCell<Vec<(String, String)>>>,
    share_cooldown: u8,
    is_multiplayer: bool,
    #[cfg(feature = "export")]
    data: Vec<ExportFrame>,
}

#[wasm_bindgen]
impl Instance {
    #[wasm_bindgen(constructor)]
    pub fn new(
        context: CanvasRenderingContext2d,
        auth_func: Function,
        backend_url: String,
    ) -> Self {
        Self {
            auth_func,
            context: Rc::new(context),
            drawing_context: DrawingContext::new(),
            input_manager: InputManager::new(),
            game: Rc::new(RefCell::new(None)),
            session: Rc::new(RefCell::new(None)),
            backend_url,
            opponent_board: Rc::new(RefCell::new(None)),
            share_cooldown: SHARE_COOLDOWN,
            messages: Rc::new(RefCell::new(Vec::with_capacity(1))),
            is_multiplayer: false,
            #[cfg(feature = "export")]
            data: Vec::new(),
        }
    }

    #[cfg(feature = "export")]
    #[wasm_bindgen]
    pub fn get_data(&self) -> Vec<u8> {
        serde_cbor::ser::to_vec_packed(&self.data).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn draw(&self) {
        const BOARD_X: f64 = 160.;
        const BOARD_Y: f64 = 60.;
        let Some(ref game) = *self.game.borrow() else {
            DrawingContext::clear(&self.context);
            return;
        };
        self.drawing_context
            .draw_board(&self.context, BOARD_X, BOARD_Y);
        self.drawing_context.draw_field(
            &self.context,
            &game.board.buffer,
            BOARD_X + 5.,
            BOARD_Y + 5.,
        );
        if !matches!(game.phase, Phase::Generation { .. }) {
            self.drawing_context.draw_tetrimino(
                &self.context,
                &game.piece,
                BOARD_X + 5.,
                BOARD_Y + 5.,
                false,
                false,
            );
            self.drawing_context.draw_tetrimino(
                &self.context,
                &game.ghost,
                BOARD_X + 5.,
                BOARD_Y + 5.,
                true,
                false,
            );
        }

        DrawingContext::draw_score(&self.context, game.score, BOARD_X, 20.);
        self.drawing_context
            .draw_hold(&self.context, game.hold.as_ref(), 20., BOARD_Y);
        self.drawing_context.draw_queue(
            &self.context,
            game.next_queue.iter(),
            BOARD_X + 350.,
            BOARD_Y,
        );
        if let Some(ref board) = *self.opponent_board.borrow() {
            DrawingContext::draw_opponent_board(
                &self.context,
                board,
                BOARD_X + 350.,
                BOARD_Y + 430.,
            );
        } else {
            DrawingContext::draw_opponent_board(
                &self.context,
                &EMPTY_BOARD,
                BOARD_X + 350.,
                BOARD_Y + 430.,
            );
        }
        DrawingContext::draw_level(&self.context, game.level, BOARD_X + 320., 20.);

        DrawingContext::draw_messages(
            &self.context,
            &self.messages.borrow(),
            BOARD_X + 350. + 160.,
            BOARD_Y,
        );
    }

    /// Should be called exaclty 60 times a second
    #[wasm_bindgen]
    #[allow(clippy::pedantic)]
    pub async fn update(&mut self, inputs: FrameInputs, move_left: bool, move_right: bool) -> bool {
        let frame_actions = self.input_manager.update(&inputs, move_left, move_right);

        let mut borrow = self.game.borrow_mut();
        let Some(ref mut game) = *borrow else {
            // if we receive start we cant start the loop from inside rust
            return true;
        };
        #[cfg(feature = "export")]
        {
            self.data.push(ExportFrame {
                board: game.board.clone(),
                piece: game.piece.clone(),
                ghost: Some(game.ghost.clone()),
                hold: game.hold.clone(),
                next_queue: game.next_queue.iter().map(|m| m.kind).collect(),
                score: game.score,
                level: game.level,
                inputs: frame_actions.clone(),
            });
        }
        game.user_actions(frame_actions);
        let events = game.events.clone();
        drop(borrow);

        self.share_cooldown -= 1;
        if self.share_cooldown == 0 {
            self.share_cooldown = SHARE_COOLDOWN;
            if let Some(ref mut session) = *self.session.borrow_mut()
                && let Some(ref game) = *self.game.borrow()
            {
                let mut board = game.board.clone();
                board.place(&game.piece);
                let _ = session.send(Message::GameState(board.into())).await;
            }
        }
        for event in &events {
            match event {
                Event::Gameover => {
                    let mut game = self.game.borrow_mut();
                    if let Some(ref mut game) = *game
                        && !game.settings.easy
                    {
                        gameover(
                            &self.backend_url,
                            &self.auth_func,
                            game.score,
                            self.is_multiplayer,
                            game.settings,
                        );
                    }
                    *game = None;
                    if let Some(mut session) = self.session.borrow_mut().take() {
                        spawn_local(async move {
                            let _ = session.send(Message::Gameover).await;
                            let _ = session.close().await;
                        });
                    }
                    return false;
                }
                Event::Completion(lines) => {
                    if let Some(ref mut session) = *self.session.borrow_mut() {
                        let lines = match lines {
                            2 => 1,
                            3 => 2,
                            4 => 4,
                            _ => 0,
                        };
                        if lines > 0 {
                            let _ = session.send(Message::LineSend(lines)).await;
                        }
                    }
                }
            }
        }
        true
    }

    #[wasm_bindgen]
    pub async fn connect(&mut self, name: &str) {
        let url = format!("{}/connect/{name}", self.backend_url);
        let ws = WsMeta::connect(&url, None).await;

        let Ok((meta, stream)) = ws else {
            return;
        };

        let framed = Framed::new(stream.into_io(), MessageCodec::new());
        let (session, stream) = framed.split();

        let session = Rc::new(RefCell::new(Some(session)));

        spawn_local(conn_loop_static(
            meta,
            stream,
            self.game.clone(),
            Rc::clone(&self.opponent_board),
            Rc::clone(&self.messages),
        ));

        self.session = session;
        self.is_multiplayer = true;
    }

    #[wasm_bindgen]
    pub fn start_singleplayer(&mut self, settings: GameSettings) -> bool {
        let config = GameConfig::default_seed(settings);
        let Ok(mut game) = self.game.try_borrow_mut() else {
            return false;
        };
        if game.is_some() {
            return false;
        }
        game.get_or_insert(Game::new(config));
        self.is_multiplayer = false;
        true
    }

    #[wasm_bindgen]
    pub fn goodbye(&self) {
        let mut game = self.game.borrow_mut();
        *game = None;
        if let Some(mut session) = self.session.borrow_mut().take() {
            spawn_local(async move {
                let _ = session.send(Message::Disconnect).await;
                let _ = session.close().await;
            });
        }
        *self.opponent_board.borrow_mut() = None;
    }
}

async fn conn_loop_static(
    _meta: WsMeta,
    mut stream: TetrisStream,
    game: Rc<RefCell<Option<Game>>>,
    opponent_board: Rc<RefCell<Option<Board>>>,
    messages: Rc<RefCell<Vec<(String, String)>>>,
) {
    while let Some(msg) = stream.next().await {
        let Ok(msg) = msg else {
            continue;
        };
        match msg {
            Message::LineSend(lines) => {
                if let Some(ref mut game) = *game.borrow_mut() {
                    game.accumulate_garbage(lines);
                }
            }
            Message::Start(settings) => {
                let try_borrow_mut = game.try_borrow_mut();
                if let Ok(mut game) = try_borrow_mut {
                    *game = Some(Game::new(settings));
                }
            }
            Message::Gameover | Message::Disconnect => {
                *opponent_board.borrow_mut() = Some(EMPTY_BOARD);
                messages.borrow_mut().push((
                    String::from("The other player has lost,\nYou win!"),
                    String::from("#0f0"),
                ));
            }
            Message::GameState(board) => {
                *opponent_board.borrow_mut() = Some(*board);
            }
        }
    }
}

fn gameover(
    backend_url: &str,
    auth_func: &Function,
    score: u32,
    is_multiplayer: bool,
    settings: GameSettings,
) {
    let window = window().unwrap();
    if !tetris_confirm("You lost!, do you want to share your score?") {
        return;
    }
    let Some(name) = tetris_prompt("Enter your name for the leaderboard:") else {
        return;
    };
    let token = auth_func
        .call1(
            &JsValue::UNDEFINED,
            &JsValue::from_str(&format!(
                "{score:o} fffffffff {name} esiovtb3w5iothbiouthes0u",
            )),
        )
        .expect("Auth function threw an error")
        .as_string()
        .expect("Auth function returned non-string");
    let options = RequestInit::new();
    options.set_method("POST");
    let headers = Headers::new().unwrap();
    let _ = headers.set("Content-Type", "application/json");
    options.set_headers(&JsValue::from(headers));
    let req = HighscoreReq {
        auth: token,
        name,
        score,
        settings,
        was_multiplayer: is_multiplayer,
    };
    options.set_body(&JsValue::from_str(
        &serde_json_wasm::to_string(&req).unwrap(),
    ));
    let _ = window.fetch_with_str_and_init(&format!("{backend_url}/highscore"), &options);
}

#![allow(clippy::future_not_send)]

use async_io_stream::IoStream;
use futures_codec::{CborCodec, Framed};
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::console_log;

use crate::{
    draw::DrawingContext,
    input::{FrameInputs, InputManager},
    tetris_confirm, tetris_prompt,
};
use js_sys::Function;
use tetris_core::{
    net::Message,
    tetris::{Event, Game, Phase},
};
use wasm_bindgen::prelude::*;
use web_sys::{window, CanvasRenderingContext2d, Headers, RequestInit};
use ws_stream_wasm::{WsMeta, WsStreamIo};

// const BACKEND_URL: &str = "https://tetris.patzl.dev";
const BACKEND_URL: &str = "http://172.21.49.178:4444";
// const BACKEND_URL: &str = "http://localhost:4444";

type MessageCodec = CborCodec<Message, Message>;
type TetrisFrames = Framed<IoStream<WsStreamIo, Vec<u8>>, MessageCodec>;

type TetrisSession = SplitSink<TetrisFrames, Message>;
type TetrisStream = SplitStream<TetrisFrames>;

#[wasm_bindgen]
pub struct Instance {
    auth_func: Function,
    context: Rc<CanvasRenderingContext2d>,
    drawing_context: DrawingContext,
    input_manager: InputManager,
    game: Rc<RefCell<Option<Game>>>,
    session: Rc<RefCell<Option<TetrisSession>>>,
}

#[wasm_bindgen]
impl Instance {
    #[wasm_bindgen(constructor)]
    pub fn new(context: CanvasRenderingContext2d, auth_func: Function) -> Self {
        Self {
            auth_func,
            context: Rc::new(context),
            drawing_context: DrawingContext::new(),
            input_manager: InputManager::new(),
            game: Rc::new(RefCell::new(None)),
            session: Rc::new(RefCell::new(None)),
        }
    }

    #[wasm_bindgen]
    pub fn draw(&self) {
        const BOARD_X: f64 = 160.;
        const BOARD_Y: f64 = 60.;
        let Some(ref game) = *self.game.borrow() else {
            self.drawing_context.clear(&self.context);
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
        DrawingContext::draw_level(&self.context, game.level, BOARD_X + 320., 20.);
    }

    /// Should be called exaclty 60 times a second
    #[wasm_bindgen]
    #[allow(clippy::pedantic)]
    pub async fn update(&mut self, inputs: FrameInputs) -> bool {
        let frame_actions = self.input_manager.update(&inputs);

        let mut borrow = self.game.borrow_mut();
        let Some(ref mut game) = *borrow else {
            // if we receive start we cant start the loop from inside rust
            return true;
        };
        game.user_actions(frame_actions);
        let events = game.events.clone();
        drop(borrow);

        for event in &events {
            match event {
                Event::Gameover => {
                    let mut game = self.game.borrow_mut();
                    if let Some(ref mut game) = *game {
                        gameover(self.auth_func.clone(), game.score);
                    }
                    *game = None;
                    if let Some(ref mut session) = *self.session.borrow_mut() {
                        let _ = session.close().await;
                    }
                    return false;
                }
                Event::Completion(lines) => {
                    if let Some(ref mut session) = *self.session.borrow_mut() {
                        let _ = session
                            .send(Message::LineSend(match lines {
                                2 => 1,
                                3 => 2,
                                4 => 4,
                                _ => 0,
                            }))
                            .await;
                    }
                }
            }
        }
        return true;
    }

    #[wasm_bindgen]
    pub async fn connect(&mut self, name: &str) {
        let url = format!("{BACKEND_URL}/connect/{name}");
        let ws = WsMeta::connect(&url, None).await;

        let Ok((meta, stream)) = ws else {
            return;
        };

        let framed = Framed::new(stream.into_io(), MessageCodec::new());
        let (session, stream) = framed.split();

        let session = Rc::new(RefCell::new(Some(session)));

        spawn_local(conn_loop_static(meta, stream, self.game.clone()));

        self.session = session;
    }
}

async fn conn_loop_static(
    _meta: WsMeta,
    mut stream: TetrisStream,
    game: Rc<RefCell<Option<Game>>>,
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
            Message::Start => {
                console_log!("Start game borrow");
                let try_borrow_mut = game.try_borrow_mut();
                if let Ok(mut game) = try_borrow_mut {
                    *game = Some(Game::new());
                } else {
                    console_log!("Cannot set game");
                }
                console_log!("Start game borrow end");
            }
        }
    }
}

fn gameover(auth_func: Function, score: u32) {
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
    options.set_body(&JsValue::from_str(&format!(
        "{{\"score\": {score}, \"auth\": \"{token}\", \"name\": \"{name}\"}}",
    )));
    let _ = window.fetch_with_str_and_init(&format!("{BACKEND_URL}/highscore"), &options);
}

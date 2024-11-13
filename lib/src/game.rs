use std::{collections::VecDeque, mem, rc::Rc};

use crate::{
    alert,
    draw::DrawingContext,
    types::{Board, Direction, Mino, Tetrimino},
};
use rand::Rng;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const LOCKDOWN_START: u8 = 30;
const SOFT_FALL_MULT: u8 = 10;
const LOCKDOWN_MOVES: u8 = 5;

#[wasm_bindgen]
pub enum Action {
    Left,
    Right,
    Cw,
    Ccw,
    Hold,
    HardDrop,
    SoftDrop,
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Generation { frames_left: u8 },
    Falling { timer: u8 },
    Lock,
    Completion,
}

#[wasm_bindgen]
pub struct Game {
    board: Board,
    piece: Tetrimino,
    ghost: Tetrimino,
    score: u32,
    level: u8,
    bag: [Mino; 7],
    bag_idx: usize,
    next_queue: VecDeque<Tetrimino>,
    context: Rc<CanvasRenderingContext2d>,
    drawing_context: DrawingContext,
    hold: Option<Tetrimino>,
    can_hold: bool,
    phase: Phase,
    lockdown_timer: u8,
    lockdown_moves: u8,
    lockdown_y: i8,
    level_goal: i8,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(context: CanvasRenderingContext2d) -> Self {
        let mut new = Self {
            board: Board::default(),
            piece: Tetrimino::new(Mino::I, 0, 0),
            ghost: Tetrimino::new(Mino::I, 0, 0),
            score: 0,
            bag: [
                Mino::O,
                Mino::I,
                Mino::J,
                Mino::L,
                Mino::S,
                Mino::Z,
                Mino::T,
            ],
            next_queue: VecDeque::with_capacity(5),
            bag_idx: 7,
            context: Rc::new(context),
            drawing_context: DrawingContext::new(),
            hold: None,
            can_hold: true,
            level: 1,
            phase: Phase::Generation { frames_left: 0 },
            lockdown_timer: LOCKDOWN_START,
            lockdown_moves: LOCKDOWN_MOVES,
            lockdown_y: 0,
            level_goal: 5,
        };
        for _ in 0..5 {
            let next_kind = new.next_kind();
            new.next_queue.push_back(Tetrimino::new(next_kind, 0, 0));
        }
        let get_next_piece = new.get_next_piece();
        new.next_piece(get_next_piece);
        new
    }

    #[wasm_bindgen]
    pub fn draw(&self) {
        const BOARD_X: f64 = 300.;
        const BOARD_Y: f64 = 20.;
        self.drawing_context
            .draw_board(&self.context, BOARD_X, BOARD_Y);
        self.drawing_context.draw_field(
            &self.context,
            &self.board.buffer,
            BOARD_X + 5.,
            BOARD_Y + 5.,
        );
        if !matches!(self.phase, Phase::Generation { .. }) {
            self.drawing_context.draw_tetrimino(
                &self.context,
                &self.piece,
                BOARD_X + 5.,
                BOARD_Y + 5.,
                false,
                false,
            );
            self.drawing_context.draw_tetrimino(
                &self.context,
                &self.ghost,
                BOARD_X + 5.,
                BOARD_Y + 5.,
                true,
                false,
            );
        }

        DrawingContext::draw_score(&self.context, self.score, 20., 20.);
        self.drawing_context
            .draw_hold(&self.context, self.hold.as_ref(), 20., 120.);
        self.drawing_context
            .draw_queue(&self.context, self.next_queue.iter(), 700., 20.);
        DrawingContext::draw_level(&self.context, self.level, 20., 200.);
    }

    /// Should be called exaclty 60 times a second
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)]
    pub fn update(&mut self, user_actions: Vec<Action>) {
        match self.phase {
            Phase::Generation { frames_left } => {
                if frames_left == 0 {
                    let piece = self.get_next_piece();
                    self.next_piece(piece);
                    self.start_fall();
                    self.can_hold = true;
                    self.lockdown_moves = LOCKDOWN_MOVES;
                } else {
                    self.phase = Phase::Generation {
                        frames_left: frames_left - 1,
                    };
                }
            }
            Phase::Falling { mut timer } => {
                self.process_input(&user_actions);
                if let Phase::Falling { timer: new_timer } = self.phase {
                    timer = new_timer;
                }
                if matches!(self.phase, Phase::Completion) {
                    return;
                }
                if timer == 0 {
                    self.board.move_down(&mut self.piece);
                    if self.piece.offset_y > self.lockdown_y {
                        self.lockdown_y = self.piece.offset_y;
                        self.lockdown_moves = LOCKDOWN_MOVES;
                    }
                    self.start_fall();
                } else {
                    self.phase = Phase::Falling { timer: timer - 1 }
                }
                if !self.board.can_move_down(&mut self.piece) {
                    self.phase = Phase::Lock;
                }
            }
            Phase::Lock => {
                if self.lockdown_timer == 0 {
                    self.board.drop(&mut self.piece);
                    self.board.place(&self.piece);
                    self.phase = Phase::Completion;
                    self.lockdown_timer = LOCKDOWN_START;
                } else {
                    self.process_input(&user_actions);
                    if self.board.can_move_down(&mut self.piece) {
                        self.start_fall();
                    } else {
                        self.lockdown_timer = self.lockdown_timer.saturating_sub(1);
                    }
                }
            }
            Phase::Completion => {
                let rows = self.board.clear_lines();
                if rows != 0 {
                    self.score += u32::from(self.level)
                        * match rows {
                            1 => 100,
                            2 => 300,
                            3 => 500,
                            4 => 800,
                            _ => 0,
                        }
                }
                self.level_goal -= rows as i8;
                if self.level_goal <= 0 {
                    self.level += 1;
                    self.level_goal += 5;
                }
                self.phase = Phase::Generation { frames_left: 12 };
            }
        }
    }
}

impl Game {
    fn process_input(&mut self, actions: &[Action]) {
        for action in actions {
            match action {
                Action::Left => {
                    let move_success = self.move_x(-1);
                    self.movement(move_success);
                }
                Action::Right => {
                    let move_success = self.move_x(1);
                    self.movement(move_success);
                }
                Action::Cw => {
                    let move_success = self.rotate(Direction::Cw);
                    self.movement(move_success);
                }
                Action::Ccw => {
                    let move_success = self.rotate(Direction::Ccw);
                    self.movement(move_success);
                }
                Action::Hold => {
                    if !self.can_hold {
                        continue;
                    }
                    self.can_hold = false;
                    if self.hold.is_none() {
                        self.hold = Some(Tetrimino::new(self.piece.kind, 0, 0));
                        let piece = self.get_next_piece();
                        self.next_piece(piece);
                    } else {
                        let piece = mem::replace(
                            &mut self.hold,
                            Some(Tetrimino::new(self.piece.kind, 0, 0)),
                        );

                        self.next_piece(piece.unwrap());
                    }
                }
                Action::HardDrop => {
                    let before = self.piece.offset_y;
                    self.board.drop(&mut self.piece);
                    self.score += 2 * (self.piece.offset_y - before).max(0) as u32;
                    self.board.place(&self.piece);
                    self.phase = Phase::Completion;
                    self.lockdown_timer = LOCKDOWN_START;
                }
                Action::SoftDrop => {
                    if let Phase::Falling { timer } = self.phase {
                        let new = timer.saturating_sub(SOFT_FALL_MULT - 1);
                        self.phase = Phase::Falling { timer: new }
                    }
                }
            }
        }
    }

    fn start_fall(&mut self) {
        self.phase = Phase::Falling {
            timer: match self.level {
                1 => 30,
                2 => 20,
                3 => 15,
                4 => 10,
                5 => 8,
                6 => 6,
                7 => 5,
                8 => 4,
                9 => 3,
                10 => 2,
                _ => 1,
            },
        }
    }

    fn movement(&mut self, move_success: bool) {
        if move_success && self.phase == Phase::Lock && self.lockdown_moves > 0 {
            self.lockdown_timer = LOCKDOWN_START;
            self.lockdown_moves -= 1;
        }
        if self.lockdown_moves == 0 {
            self.lockdown_timer = 0;
        }
    }

    fn move_x(&mut self, offset: i8) -> bool {
        let success = self.board.move_x(&mut self.piece, offset);
        self.update_ghost();
        success
    }

    fn rotate(&mut self, direction: Direction) -> bool {
        let success = self.board.rotate(&mut self.piece, direction);
        self.update_ghost();
        success
    }

    fn update_ghost(&mut self) {
        let mut clone = self.piece.clone();
        self.board.drop(&mut clone);
        self.ghost = clone;
    }

    fn next_kind(&mut self) -> Mino {
        if self.bag_idx < 7 {
            let next = self.bag[self.bag_idx];
            self.bag_idx += 1;
            return next;
        }
        let mut rand = rand::thread_rng();
        for i in 0..7 {
            let swap = rand.gen_range(i..7);
            self.bag.swap(i, swap);
        }
        self.bag_idx = 1;
        self.bag[0]
    }

    fn place_next_piece(tetrimino: &mut Tetrimino) {
        let (x, y) = match tetrimino.kind {
            Mino::Empty => (0, 0),
            Mino::O => (4, 18),
            Mino::I => (3, 19),
            Mino::L | Mino::J | Mino::S | Mino::Z | Mino::T => (3, 18),
        };
        tetrimino.offset_x = x;
        tetrimino.offset_y = y;
    }

    fn next_piece(&mut self, mut piece: Tetrimino) {
        Self::place_next_piece(&mut piece);
        if !self.board.can_place(&piece) {
            return self.gameover();
        }
        let moved = self.board.move_down(&mut piece);
        if !moved {
            return self.gameover();
        }
        let mut ghost = piece.clone();
        self.board.drop(&mut ghost);
        // no drop - lock down above playfield
        if ghost.offset_x == piece.offset_x && ghost.offset_y == piece.offset_y {
            return self.gameover();
        }

        self.ghost = ghost;
        self.lockdown_y = piece.offset_y;
        self.piece = piece;
    }

    fn get_next_piece(&mut self) -> Tetrimino {
        let piece = self
            .next_queue
            .pop_front()
            .expect_throw("Next queue was empty");
        let kind = self.next_kind();
        self.next_queue.push_back(Tetrimino::new(kind, 0, 0));
        piece
    }

    #[allow(clippy::pedantic)]
    fn gameover(&self) {
        alert("Verloren");
    }
}

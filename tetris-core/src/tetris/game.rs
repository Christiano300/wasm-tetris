#[cfg(feature = "wasm-bindgen")]
use wasm_bindgen::prelude::wasm_bindgen;

use rand::{SeedableRng, prelude::Rng, rngs::SmallRng};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::{Board, Direction, Mino, Tetrimino};

const LOCKDOWN_START: u8 = 30;
const SOFT_FALL_MULT: u8 = 10;
const LOCKDOWN_MOVES: u8 = 5;
const LEVEL_GOAL: i8 = 5;

pub type RandomSeed = [u8; 32];

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "wasm-bindgen", wasm_bindgen)]
pub struct GameSettings {
    pub jupiter: bool,
    pub easy: bool,
    pub nes: bool,
    pub random: bool,
}

#[cfg_attr(feature = "wasm-bindgen", wasm_bindgen)]
impl GameSettings {
    #[cfg_attr(feature = "wasm-bindgen", wasm_bindgen(constructor))]
    pub fn new(jupiter: bool, easy: bool, nes: bool, random: bool) -> Self {
        Self {
            jupiter,
            easy,
            nes,
            random,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameConfig {
    pub settings: GameSettings,
    pub seed: Option<RandomSeed>,
}

impl GameConfig {
    pub fn default_seed(settings: GameSettings) -> Self {
        Self {
            settings,
            seed: None,
        }
    }

    pub fn with_seed(settings: GameSettings, seed: RandomSeed) -> Self {
        Self {
            settings,
            seed: Some(seed),
        }
    }
}

pub fn getrandom(seed: Option<RandomSeed>) -> SmallRng {
    match seed {
        Some(seed) => SmallRng::from_seed(seed),
        None => SmallRng::from_os_rng(),
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Event {
    Completion(u8),
    Gameover,
}

#[derive(PartialEq, Eq, Debug)]
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
pub enum Phase {
    Generation { frames_left: u8 },
    Falling { timer: u8 },
    Lock,
    Completion,
}

#[derive(Debug)]
pub struct Game {
    pub board: Board,
    pub piece: Tetrimino,
    pub ghost: Tetrimino,
    pub score: u32,
    pub level: u8,
    bag: [Mino; 7],
    bag_idx: usize,
    pub next_queue: VecDeque<Tetrimino>,
    pub hold: Option<Tetrimino>,
    can_hold: bool,
    pub phase: Phase,
    lockdown_timer: u8,
    lockdown_moves: u8,
    lockdown_y: i8,
    level_goal: i8,
    piece_rng: SmallRng,
    garbage_rng: SmallRng,
    pub done: bool,
    pub garbage_slot: u8,
    pub garbage_acc: u8,
    pub events: Vec<Event>,
    pub settings: GameSettings,
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
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
            hold: None,
            can_hold: true,
            level: 1,
            phase: Phase::Generation { frames_left: 0 },
            lockdown_timer: LOCKDOWN_START,
            lockdown_moves: LOCKDOWN_MOVES,
            lockdown_y: 0,
            level_goal: LEVEL_GOAL,
            piece_rng: getrandom(config.seed),
            garbage_rng: getrandom(config.seed),
            done: false,
            events: vec![],
            garbage_slot: 0,
            garbage_acc: 0,
            settings: config.settings,
        };
        let mut rng = getrandom(config.seed);
        new.garbage_slot = rng.random_range(1..9);

        for _ in 0..5 {
            let next_kind = new.next_kind();
            new.next_queue.push_back(Tetrimino::new(next_kind, 0, 0));
        }
        let get_next_piece = new.get_next_piece();
        new.next_piece(get_next_piece);
        new
    }

    pub fn user_actions(&mut self, user_actions: Vec<Action>) {
        self.events.clear();
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
                    if user_actions.contains(&Action::SoftDrop) {
                        self.score += self.level as u32;
                    }
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
                if self.settings.nes || self.lockdown_timer == 0 {
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
                self.score += u32::from(self.level)
                    * match rows {
                        1 => 100,
                        2 => 300,
                        3 => 550,
                        4 => 800,
                        _ => 0,
                    };
                self.events.push(Event::Completion(rows));
                if !self.settings.easy {
                    self.level_goal -= rows as i8;
                    if self.level_goal <= 0 {
                        self.level = self.level.saturating_add(1);
                        self.level_goal += LEVEL_GOAL;
                    }
                }
                self.add_garbage();
                self.phase = Phase::Generation { frames_left: 12 };
            }
        }
    }

    pub fn accumulate_garbage(&mut self, lines: u8) {
        self.garbage_acc += lines;
    }

    pub fn skip_completion(&mut self) {
        if let Phase::Completion = self.phase {
            self.user_actions(vec![]);
            self.phase = Phase::Generation { frames_left: 0 };
        }
    }

    fn add_garbage(&mut self) {
        self.board.push_up(self.garbage_acc);
        for i in 0..self.garbage_acc {
            let layer = 40 - i - 1;
            self.board.add_garbage(layer, self.garbage_slot);
            if self.garbage_rng.random_bool(0.3) {
                self.garbage_slot = self.garbage_rng.random_range(0..10);
            }
        }
        self.garbage_acc = 0;
    }

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
                        let piece = self.hold.replace(Tetrimino::new(self.piece.kind, 0, 0));

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
        if self.settings.jupiter
            && !actions.contains(&Action::SoftDrop)
            && let Phase::Falling { timer } = self.phase
        {
            let new = timer.saturating_sub(SOFT_FALL_MULT - 1);
            self.phase = Phase::Falling { timer: new }
        }
    }

    const fn start_fall(&mut self) {
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
        if self.settings.random {
            return self.bag[self.piece_rng.random_range(0..7)];
        }
        if self.bag_idx < 7 {
            let next = self.bag[self.bag_idx];
            self.bag_idx += 1;
            return next;
        }
        for i in 0..7 {
            let swap = self.piece_rng.random_range(i..7);
            self.bag.swap(i, swap);
        }
        self.bag_idx = 1;
        self.bag[0]
    }

    const fn place_next_piece(tetrimino: &mut Tetrimino) {
        let (x, y) = match tetrimino.kind {
            Mino::Empty | Mino::Garbage => (0, 0),
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
        let piece = self.next_queue.pop_front().expect("Next queue was empty");
        let kind = self.next_kind();
        self.next_queue.push_back(Tetrimino::new(kind, 0, 0));
        piece
    }

    fn gameover(&mut self) {
        self.done = true;
        self.events.push(Event::Gameover);
    }
}

#![allow(dead_code)]
use burn::prelude::*;
use tetris_core::tetris::{
    Action as TetrisAction, BOARD_HEIGHT as FULL_BOARD_HEIGHT, BOARD_WIDTH, Game, Mino,
    Rotation as PieceRotation,
};

use crate::model::{BOARD_CHANNELS, BOARD_HEIGHT, PIECE_INPUTS};

pub const STATE_SIZE: usize = BOARD_CHANNELS * BOARD_HEIGHT * BOARD_WIDTH + PIECE_INPUTS;
pub type StateVec = [f32; STATE_SIZE];

pub fn state_vec_to_tensor<B: Backend>(state: &StateVec, device: &B::Device) -> Tensor<B, 1> {
    Tensor::from_floats(*state, device)
}

pub fn to_tensor<B: Backend>(game: &Game, device: &B::Device) -> Tensor<B, 1> {
    let state_vec = to_state(game);
    Tensor::from_floats(state_vec, device)
}

pub fn to_state(game: &Game) -> StateVec {
    let mut state = [0.0; STATE_SIZE];
    let mut input_vec: Vec<f32> = Vec::with_capacity(STATE_SIZE);

    // Board representation
    for row in game
        .board
        .buffer
        .iter()
        .skip(FULL_BOARD_HEIGHT - BOARD_HEIGHT)
    {
        for &cell in row.iter() {
            input_vec.push(mino_mapping(cell));
        }
    }

    // Current piece representation
    let mut current_piece_layer = vec![mino_mapping(Mino::Empty); BOARD_HEIGHT * BOARD_WIDTH];
    let piece = &game.piece;
    for (y, row) in piece.grid.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let index = x
                + piece.offset_x as usize
                + (y + piece.offset_y as usize + BOARD_HEIGHT - FULL_BOARD_HEIGHT) * BOARD_WIDTH;
            if index >= current_piece_layer.len() {
                continue;
            }
            current_piece_layer[index] = mino_mapping(if *cell { piece.kind } else { Mino::Empty });
        }
    }
    input_vec.extend(current_piece_layer);

    // Ghost piece representation
    let mut ghost_piece_layer = vec![mino_mapping(Mino::Empty); BOARD_HEIGHT * BOARD_WIDTH];
    let ghost_y = game.ghost.offset_y;
    for (y, row) in piece.grid.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let index = x
                + piece.offset_x as usize
                + (y + ghost_y as usize + BOARD_HEIGHT - FULL_BOARD_HEIGHT) * BOARD_WIDTH;
            if index >= ghost_piece_layer.len() {
                continue;
            }
            ghost_piece_layer[index] = mino_mapping(if *cell { piece.kind } else { Mino::Empty });
        }
    }
    input_vec.extend(ghost_piece_layer);

    // Hold piece representation
    let hold_value = match &game.hold {
        Some(mino) => mino_mapping(mino.kind),
        None => 0.0,
    };
    input_vec.push(hold_value);

    // Next queue representation
    for next in game.next_queue.iter().take(5) {
        input_vec.push(mino_mapping(next.kind));
    }

    for (i, &val) in input_vec.iter().enumerate() {
        state[i] = val;
    }
    state
}

fn mino_mapping(mino: Mino) -> f32 {
    match mino {
        Mino::Empty => 0.0,
        Mino::I => 1.0,
        Mino::O => 2.0,
        Mino::T => 3.0,
        Mino::S => 4.0,
        Mino::Z => 5.0,
        Mino::J => 6.0,
        Mino::L => 7.0,
        Mino::Garbage => 8.0,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Action {
    pub horizontal: HorizontalMove,
    pub rotation: Rotation,
    pub action: PieceAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalMove {
    Left = 0,
    Right = 1,
    None = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Cw = 3,
    Ccw = 4,
    None = 5,
}

#[derive(Debug, Clone, Copy)]
pub enum PieceAction {
    HardDrop = 6,
    SoftDrop = 7,
    Hold = 8,
    None = 9,
}

impl Action {
    pub fn from_output<B: Backend>(output: Tensor<B, 1>) -> Self {
        let mut chunks = output.split_with_sizes(vec![3, 3, 4], 0);

        let horizontal = match chunks.remove(0).argmax(0).into_scalar().to_i32() {
            0 => HorizontalMove::Left,
            1 => HorizontalMove::Right,
            _ => HorizontalMove::None,
        };

        let rotation = match chunks.remove(0).argmax(0).into_scalar().to_i32() {
            0 => Rotation::Cw,
            1 => Rotation::Ccw,
            _ => Rotation::None,
        };

        let action = match chunks.remove(0).argmax(0).into_scalar().to_i32() {
            0 => PieceAction::HardDrop,
            1 => PieceAction::SoftDrop,
            2 => PieceAction::Hold,
            _ => PieceAction::None,
        };

        Self {
            horizontal,
            rotation,
            action,
        }
    }

    pub fn from_batch<B: Backend>(outputs: Tensor<B, 2>) -> Vec<Self> {
        outputs
            .iter_dim(0)
            .map(|output| output.squeeze())
            .map(Self::from_output)
            .collect::<Vec<_>>()
    }

    pub fn no_op() -> Self {
        Self {
            horizontal: HorizontalMove::None,
            rotation: Rotation::None,
            action: PieceAction::None,
        }
    }

    pub fn random() -> Action {
        use rand::Rng;
        let mut rng = rand::rng();

        let horizontal = match rng.random_range(0..=2) {
            0 => HorizontalMove::Left,
            1 => HorizontalMove::Right,
            _ => HorizontalMove::None,
        };

        let rotation = match rng.random_range(0..=3) {
            0 => Rotation::Cw,
            1 => Rotation::Ccw,
            _ => Rotation::None,
        };

        let do_something = rotation == Rotation::None && horizontal == HorizontalMove::None;
        let action = match rng.random_range(0..=(if do_something { 3 } else { 8 })) {
            0 => PieceAction::HardDrop,
            1..=2 => PieceAction::SoftDrop,
            3 => PieceAction::Hold,
            _ => PieceAction::None,
        };

        Self {
            horizontal,
            rotation,
            action,
        }
    }

    pub fn to_tensor(&self) -> [i32; 10] {
        let mut tensor = [0; 10];

        tensor[self.horizontal as usize] = 1;
        tensor[self.rotation as usize] = 1;
        tensor[self.action as usize] = 1;

        tensor
    }

    pub fn get_logit_values<B: Backend>(next_state_logits: Tensor<B, 2>) -> Tensor<B, 2> {
        let indices = next_state_logits.clone().argmax(1);
        next_state_logits.gather(1, indices.unsqueeze())
    }
}

#[derive(Debug)]
pub struct Snapshot {
    pub state: StateVec,
    pub reward: f32,
    pub done: bool,
}

struct Previous {
    pub score: u32,
    pub level: u8,
    pub piece_x: i8,
    pub piece_y: i8,
    pub piece_rotation: PieceRotation,
}

impl Previous {
    pub fn get(game: &Game) -> Self {
        Self {
            score: game.score,
            level: game.level,
            piece_x: game.piece.offset_x,
            piece_y: game.piece.offset_y,
            piece_rotation: game.piece.rotation,
        }
    }
}

pub fn step_inner(game: &mut Game, action: &Action) -> (f32, bool) {
    let mut actions = Vec::with_capacity(3);
    match action.horizontal {
        HorizontalMove::Left => {
            actions.push(TetrisAction::Left);
        }
        HorizontalMove::Right => {
            actions.push(TetrisAction::Right);
        }
        HorizontalMove::None => {}
    }

    match action.rotation {
        Rotation::Cw => {
            actions.push(TetrisAction::Cw);
        }
        Rotation::Ccw => {
            actions.push(TetrisAction::Ccw);
        }
        Rotation::None => {}
    }

    match action.action {
        PieceAction::HardDrop => {
            actions.push(TetrisAction::HardDrop);
        }
        PieceAction::SoftDrop => {
            actions.push(TetrisAction::SoftDrop);
        }
        PieceAction::Hold => {
            actions.push(TetrisAction::Hold);
        }
        PieceAction::None => {}
    }

    let previous = Previous::get(game);

    game.user_actions(actions);
    game.skip_completion();

    (get_reward(game, action, previous), game.done)
}

fn get_mino_shapes(mino: Mino) -> &'static [&'static [i8]] {
    match mino {
        Mino::Empty | Mino::Garbage | Mino::I => &[],
        Mino::O => &[&[0]],
        Mino::J => &[&[0], &[0, 0], &[0, -1], &[2]],
        Mino::L => &[&[0], &[0, 0], &[1, 1], &[-2]],
        Mino::S => &[&[0, 1], &[-1]],
        Mino::Z => &[&[-1, -1], &[1]],
        Mino::T => &[&[1], &[-1], &[-1, 0], &[0, 0]],
    }
}

fn get_reward(game: &Game, action: &Action, prev: Previous) -> f32 {
    if game.done {
        return 0.0;
    }

    let mut reward = 0.0;

    // Reward for score increase
    // let score_diff = game.score.saturating_sub(prev.score);
    // reward += (score_diff / prev.level as u32) as f32;

    // Calculate highest point for each column
    let mut heights = [0; BOARD_WIDTH];
    for (x, height) in heights.iter_mut().enumerate() {
        for y in 0..FULL_BOARD_HEIGHT {
            if game.board.buffer[y][x] != Mino::Empty {
                *height = FULL_BOARD_HEIGHT - y;
                break;
            }
        }
    }

    let max_height = *heights.iter().max().unwrap();
    reward -= (max_height as isize - 15).max(0) as f32 * 0.1;

    let avg_height = heights.iter().map(|&h| h as f32).sum::<f32>() / BOARD_WIDTH as f32;
    reward -= (avg_height - 15.0).max(0.0) * 0.3;

    if !matches!(action.rotation, Rotation::None) {
        if prev.piece_rotation == game.piece.rotation {
            reward -= 10.0; // Penalty for invalid rotation
        }
    } else if !matches!(action.horizontal, HorizontalMove::None)
        && prev.piece_x == game.piece.offset_x
    {
        reward -= 10.0; // Penalty for invalid horizontal move
    }

    const MAX_PLACE_SCORE: i32 = 3;
    const PLACE_SCORE_MULT: f32 = 1. / 15.;
    for (idx, piece) in game.next_queue.iter().take(5).enumerate() {
        let mut places = 0;
        let value = match idx {
            0 => 5,
            1 => 2,
            _ => 1,
        };
        if piece.kind == Mino::I {
            reward += (value * MAX_PLACE_SCORE) as f32 * PLACE_SCORE_MULT;
            continue;
        }
        for col in 0..(BOARD_WIDTH - 1) {
            let orig = heights[col];
            let shapes = get_mino_shapes(piece.kind);
            for shape in shapes.iter() {
                let mut can_place = true;
                for (dx, &dy) in shape.iter().enumerate() {
                    let x = col + dx + 1;
                    if x >= BOARD_WIDTH {
                        can_place = false;
                        break;
                    }
                    let target_height = orig as i8 + dy;
                    if target_height != heights[x] as i8 {
                        can_place = false;
                        break;
                    }
                }
                if can_place {
                    places += 1;
                    break;
                }
            }
        }

        let place_score = match places {
            0 => -30,
            1 => -5,
            2 => 0,
            3 => 1,
            4 => 2,
            _ => MAX_PLACE_SCORE,
        };
        reward += (value * place_score) as f32 * PLACE_SCORE_MULT;
    }

    #[allow(clippy::needless_range_loop)]
    for col in 0..BOARD_WIDTH {
        for row in (heights[col] + 1)..FULL_BOARD_HEIGHT {
            if game.board.buffer[row][col] == Mino::Empty {
                reward -= 0.2; // Penalty for holes
            }
        }
    }

    let offset = (game.piece.offset_x - 3).abs().pow(2) as f32;
    reward += offset * 2.0;

    reward
}

#![recursion_limit = "256"]
mod dqn;
mod environment;
mod memory;
/// Inputs to model:
/// Board: bottom 24 rows with mino (0 = empty)
/// Current piece: Same as board
/// Ghost: Same as board
/// Hold: one number: mino
/// Next queue: five minos
///
/// Outputs from model:
/// Left/Right/None softmax
/// Cw/Ccw/None softmax
/// Hard Drop/Soft Drop/Hold/None softmax
///
/// Things to take into account for evaluating
/// - Holes
/// - Can place the next minos safely
/// - Score
/// - Invalid moves (Hold, Invalid rotation)
/// - Gameover
/// - Max + 2nd Height
/// - Avg Height
/// - Just the possibility of a tetris
///
mod model;
mod ppo;
mod tetris;
mod tui;

use crate::{
    dqn::{DQNConfig, RandomActionThresholdConfig, train_loop},
    model::Model,
};
use burn::{
    backend::{Autodiff, Wgpu},
    module::Module,
    record::{DefaultFileRecorder, FullPrecisionSettings},
};

fn main() -> Result<(), burn::record::RecorderError> {
    type MyBackend = Wgpu<f32, i32>;
    type MyAutodiffBackend = Autodiff<MyBackend>;

    let device = burn::backend::wgpu::WgpuDevice::default();
    let model: Model<MyAutodiffBackend> = train_loop(
        100_000,
        500,
        &device,
        DQNConfig::new(0.999, 0.05, 0.01, 32),
        RandomActionThresholdConfig::new(0.9, 0.05, 50.),
    );
    model.save_file(
        "./model_saved",
        &DefaultFileRecorder::<FullPrecisionSettings>::default(),
    )
}

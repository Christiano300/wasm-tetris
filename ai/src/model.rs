use std::{any::Any, collections::HashMap};

use burn::{
    module::{Module, ModuleMapper, ModuleVisitor, Param, ParamId},
    nn::{
        Dropout, DropoutConfig, LeakyRelu, LeakyReluConfig, Linear, LinearConfig, PaddingConfig2d,
        conv::{Conv2d, Conv2dConfig},
        pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig, MaxPool2d, MaxPool2dConfig},
    },
    prelude::*,
};

pub const BOARD_HEIGHT: usize = 24;
pub const BOARD_WIDTH: usize = 10;
pub const BOARD_CHANNELS: usize = 3;

const OUTPUT_MOVE: usize = 3;
const OUTPUT_ROTATE: usize = 3;
const OUTPUT_ACTION: usize = 4;
const OUTPUT_TOTAL: usize = OUTPUT_MOVE + OUTPUT_ROTATE + OUTPUT_ACTION;

const QUEUE_SIZE: usize = 5;
pub const PIECE_INPUTS: usize = QUEUE_SIZE + 1;

#[derive(Config, Debug)]
pub struct ModelConfig {
    #[config(default = "0.1")]
    dropout: f64,

    #[config(default = "[32, 16]")]
    hidden_size: [usize; 2],
}

impl ModelConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            conv1: Conv2dConfig::new([3, 6], [1, 1])
                .with_padding(PaddingConfig2d::Same)
                .init(device),
            conv2: Conv2dConfig::new([6, 8], [3, 3])
                .with_padding(PaddingConfig2d::Same)
                .init(device),
            conv3: Conv2dConfig::new([8, 8], [3, 3])
                .with_padding(PaddingConfig2d::Valid)
                .init(device),
            pool1: AdaptiveAvgPool2dConfig::new([8, 8]).init(),
            conv4: Conv2dConfig::new([8, 16], [3, 3])
                .with_padding(PaddingConfig2d::Valid)
                .init(device),
            pool2: MaxPool2dConfig::new([2, 2]).init(),
            // channels * width * height + hold + queue
            linear1: LinearConfig::new(16 * 3 * 3 + PIECE_INPUTS, self.hidden_size[0]).init(device),
            linear2: LinearConfig::new(self.hidden_size[0], self.hidden_size[1]).init(device),
            // move + rotation + drop/hold
            linear3: LinearConfig::new(self.hidden_size[1], OUTPUT_TOTAL).init(device),
            activation: LeakyReluConfig::new().with_negative_slope(0.1).init(),
            dropout: DropoutConfig::new(self.dropout).init(),
        }
    }
}

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
    conv3: Conv2d<B>,
    conv4: Conv2d<B>,
    pool1: AdaptiveAvgPool2d,
    pool2: MaxPool2d,
    linear1: Linear<B>,
    linear2: Linear<B>,
    linear3: Linear<B>,
    activation: LeakyRelu,
    dropout: Dropout,
}

impl<B: Backend> Model<B> {
    /// # Shapes
    /// - Board: \[2, 24, 10]
    /// - Pieces: \[1 + 5]
    /// - Output: \[3 + 3 + 4]
    pub fn forward(&self, state: Tensor<B, 2>) -> Tensor<B, 2> {
        let mut parts = state.split_with_sizes(
            vec![BOARD_CHANNELS * BOARD_HEIGHT * BOARD_WIDTH, PIECE_INPUTS],
            1,
        );
        let x = parts.remove(0).reshape([
            -1,
            BOARD_CHANNELS as i32,
            BOARD_HEIGHT as i32,
            BOARD_WIDTH as i32,
        ]);

        let x = self.conv1.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);
        let x = self.conv2.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);
        let x = self.conv3.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);
        let x = self.pool1.forward(x);
        let x = self.conv4.forward(x);
        let x = self.dropout.forward(x);
        let x = self.pool2.forward(x);

        let x = x.reshape([-1, 16 * 3 * 3]);
        let x = Tensor::cat(vec![x, parts.remove(0)], 1);
        let x = self.linear1.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);
        let x = self.linear2.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);
        let x = self.linear3.forward(x);
        let x = self.activation.forward(x);

        x
    }

    pub fn soft_update(self, policy_net: &Model<B>, tau: f32) -> Self {
        struct Visitor(HashMap<ParamId, Box<dyn Any>>);

        impl<B: Backend> ModuleVisitor<B> for Visitor {
            fn visit_float<const D: usize>(&mut self, param: &Param<Tensor<B, D>>) {
                self.0.insert(param.id.clone(), Box::new(param.val()));
            }
        }

        struct Mapper(HashMap<ParamId, Box<dyn Any>>, f32);

        impl<B: Backend> ModuleMapper<B> for Mapper {
            fn map_float<const D: usize>(
                &mut self,
                param: Param<Tensor<B, D>>,
            ) -> Param<Tensor<B, D>> {
                let other = self
                    .0
                    .get(&param.id)
                    .expect("Falied to do soft update: param not found");
                let other = other
                    .downcast_ref::<Tensor<B, D>>()
                    .expect("Falied to do soft update: param has wrong type")
                    .clone();
                let new_val = param.val() * (1.0 - self.1) + other * self.1;
                Param::initialized(param.id.clone(), new_val)
            }
        }

        let mut visitor = Visitor(HashMap::new());
        policy_net.visit(&mut visitor);
        let mut mapper = Mapper(visitor.0, tau);
        self.map(&mut mapper)
    }
}

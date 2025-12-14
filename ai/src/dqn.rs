use burn::{
    Tensor,
    config::Config,
    grad_clipping::GradientClippingConfig,
    nn::loss::{HuberLossConfig, Reduction},
    optim::{AdamWConfig, GradientsParams, Optimizer},
    prelude::Backend,
    tensor::backend::AutodiffBackend,
};
use rand::random;
use replace_with::replace_with_or_abort;

use crate::{
    environment::Environment,
    memory::Memory,
    model::{Model, ModelConfig},
    tetris::Action,
    tui::{Event, Stat, Tui},
};

#[derive(Config, Debug)]
pub struct DQNConfig {
    pub gamma: f32,
    pub tau: f32,
    pub learning_rate: f32,
    pub batch_size: usize,
}

impl DQNConfig {
    pub fn init<B: AutodiffBackend>(self, device: &B::Device) -> DQN<B> {
        DQN::new(device, self)
    }
}

pub struct DQN<B: Backend> {
    policy_net: Model<B>,
    target_net: Model<B>,
    pub config: DQNConfig,
}

impl<B: AutodiffBackend> DQN<B> {
    fn new(device: &B::Device, config: DQNConfig) -> Self {
        let policy_net = ModelConfig::new().init(device);
        let target_net = policy_net.clone();
        Self {
            policy_net,
            target_net,
            config,
        }
    }

    pub fn select_action(&self, state: Tensor<B, 1>, random_threshold: f64) -> Action {
        if random::<f64>() > random_threshold {
            let q_values = self.policy_net.forward(state.unsqueeze());
            Action::from_batch(q_values)[0]
        } else {
            Action::random()
        }
    }

    pub fn train(
        &mut self,
        memory: &Memory,
        optimizer: &mut (impl Optimizer<Model<B>, B> + Sized),
    ) -> f32 {
        let (states, actions, rewards, next_states, not_dones) =
            memory.get_samples::<B>(self.config.batch_size);

        let state_logits = self.policy_net.forward(states);
        let state_action_logits = state_logits.gather(1, actions);

        let next_state_logits = self.target_net.forward(next_states);
        let next_action_logits = Action::get_logit_values(next_state_logits);

        let expected_state_action_logits =
            rewards + not_dones * next_action_logits * self.config.gamma;

        let loss = HuberLossConfig::new(1.0).init().forward(
            state_action_logits,
            expected_state_action_logits,
            Reduction::Mean,
        );

        let gradients = loss.backward();
        let gradient_params = GradientsParams::from_grads(gradients, &self.policy_net);
        replace_with_or_abort(&mut self.policy_net, |policy_net| {
            optimizer.step(
                self.config.learning_rate.into(),
                policy_net,
                gradient_params,
            )
        });

        replace_with_or_abort(&mut self.target_net, |target_net| {
            target_net.soft_update(&self.policy_net, self.config.tau)
        });

        loss.to_data().as_slice().unwrap()[0]
    }
}

#[derive(Config, Debug)]
pub struct RandomActionThresholdConfig {
    /// The threshold for the first episode
    pub start: f64,

    /// The infinite limit of the threshold
    pub end: f64,

    /// The decay rate, aka. how many episodes until the threshold multiplied by 1/e
    pub decay: f64,
}

impl RandomActionThresholdConfig {
    pub fn get_threshold(&self, episode: usize) -> f64 {
        self.end + (self.start - self.end) * f64::exp(-(episode as f64) / self.decay)
    }
}

pub fn train_loop<B: AutodiffBackend>(
    num_episodes: usize,
    max_steps: usize,
    device: &B::Device,
    config: DQNConfig,
    random_action: RandomActionThresholdConfig,
) -> Model<B> {
    let mut model = config.clone().init(device);

    let mut env = Environment::new();

    let mut memory = Memory::default();

    let mut tui = Tui::init();

    let mut optimizer = AdamWConfig::new()
        .with_grad_clipping(Some(GradientClippingConfig::Value(100.0)))
        .init();

    for episode in 0..num_episodes {
        memory.clear();
        let mut episode_done = false;
        let mut episode_reward: f32 = 0.0;
        let mut episode_duration = 0_usize;
        let mut state = env.state();
        let mut episode_loss = 0.0;
        let mut loss_iterations = 0.0;
        let eps_threshold = random_action.get_threshold(episode);

        while !episode_done {
            let action = model.select_action(Tensor::from_floats(state, device), eps_threshold);
            let snapshot = env.step(&action);

            episode_reward += snapshot.reward;

            memory.add(
                state,
                action,
                snapshot.reward,
                snapshot.state,
                snapshot.done,
            );

            let loss = if config.batch_size <= memory.len() {
                loss_iterations += 1.0;
                Some(model.train(&memory, &mut optimizer))
            } else {
                None
            };
            episode_loss += loss.unwrap_or(0.0);

            if let Some(event) = tui.render(&env, loss, snapshot.reward) {
                match event {
                    Event::Quit => {
                        tui.end();
                        return model.target_net;
                    }
                }
            }

            episode_duration += 1;

            if snapshot.done || episode_duration >= max_steps {
                env.reset();
                episode_done = true;

                tui.new_stat(Stat {
                    episode,
                    reward: episode_reward,
                    steps: episode_duration,
                    reward_per_step: episode_reward / (episode_duration as f32),
                    loss: episode_loss / (loss_iterations as f32),
                });
            } else {
                state = snapshot.state;
            }
        }
    }

    model.target_net
}

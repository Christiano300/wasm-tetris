use crate::tetris::{Action, StateVec};
use burn::{
    Tensor,
    prelude::Backend,
    tensor::{BasicOps, Element, Int, TensorKind},
};
use rand::{Rng, distr::Uniform};

#[derive(Debug, Default)]
pub struct Memory {
    pub state: Vec<StateVec>,
    pub action: Vec<Action>,
    pub reward: Vec<f32>,
    pub next_state: Vec<StateVec>,
    pub done: Vec<bool>,
}

impl Memory {
    pub fn add(
        &mut self,
        state: StateVec,
        action: Action,
        reward: f32,
        next_state: StateVec,
        done: bool,
    ) {
        self.state.push(state);
        self.action.push(action);
        self.reward.push(reward);
        self.next_state.push(next_state);
        self.done.push(done);
    }

    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn clear(&mut self) {
        self.state.clear();
        self.action.clear();
        self.reward.clear();
        self.next_state.clear();
        self.done.clear();
    }

    pub fn get_samples<B: Backend>(
        &self,
        batch_size: usize,
    ) -> (
        Tensor<B, 2>,
        Tensor<B, 2, Int>,
        Tensor<B, 2>,
        Tensor<B, 2>,
        Tensor<B, 2>,
    ) {
        let indices = rand::rng()
            .sample_iter(Uniform::new(0, self.len()).unwrap())
            .take(batch_size)
            .collect::<Vec<_>>();
        let states = Self::to_tensor(&self.state, &indices, |s| s);
        let actions = Self::to_tensor_owned::<_, _, _, _, Int>(&self.action, &indices, |a| {
            a.to_tensor().into()
        });
        let rewards = Self::to_tensor_owned(&self.reward, &indices, |r| vec![*r]);
        let next_states = Self::to_tensor(&self.next_state, &indices, |s| s);
        let not_dones =
            Self::to_tensor_owned(&self.done, &indices, |d| vec![if *d { 0.0 } else { 1.0 }]);
        (states, actions, rewards, next_states, not_dones)
    }

    fn to_tensor<B, T, M, D>(floats: &Vec<T>, indices: &[usize], mapper: M) -> Tensor<B, 2>
    where
        B: Backend,
        M: Fn(&T) -> &[D],
        D: Element,
    {
        let nums = indices
            .iter()
            .flat_map(|&i| mapper(&floats[i]))
            .copied()
            .collect::<Vec<_>>();
        Tensor::<B, 1>::from_data(&nums[..], &Default::default())
            .reshape([indices.len() as i32, -1])
    }

    fn to_tensor_owned<B, T, M, D, K>(
        floats: &Vec<T>,
        indices: &[usize],
        mapper: M,
    ) -> Tensor<B, 2, K>
    where
        B: Backend,
        M: Fn(&T) -> Vec<D>,
        D: Element,
        K: TensorKind<B> + BasicOps<B>,
    {
        let nums = indices
            .iter()
            .flat_map(|&i| mapper(&floats[i]))
            .collect::<Vec<_>>();
        Tensor::<B, 1, K>::from_data(&nums[..], &Default::default())
            .reshape([indices.len() as i32, -1])
    }
}

use wasm_bindgen::prelude::*;

const DAS_DELAY: u8 = 15;
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

#[wasm_bindgen]
pub struct FrameInputs {
    left: bool,
    right: bool,
    cw: bool,
    ccw: bool,
    hold: bool,
    hard_drop: bool,
    soft_drop: bool,
}

#[wasm_bindgen]
impl FrameInputs {
    #[wasm_bindgen(constructor)]
    pub fn new(
        left: bool,
        right: bool,
        cw: bool,
        ccw: bool,
        hold: bool,
        hard_drop: bool,
        soft_drop: bool,
    ) -> Self {
        Self {
            left,
            right,
            cw,
            ccw,
            hold,
            hard_drop,
            soft_drop,
        }
    }
}

pub struct InputManager {
    left_frames: u8,
    right_frames: u8,
    cw: u8,
    ccw: u8,
}

macro_rules! input {
    ($input:expr, $counter:expr, $actions:ident, $action:ident) => {
        if $input {
            $counter += 1;
            if $counter == 1 || $counter > DAS_DELAY {
                $actions.push(Action::$action);
            }
        } else {
            $counter = 0;
        }
    };
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            left_frames: 0,
            right_frames: 0,
            cw: 0,
            ccw: 0,
        }
    }

    pub fn update(&mut self, inputs: FrameInputs) -> Vec<Action> {
        let mut actions = vec![];

        input!(inputs.left, self.left_frames, actions, Left);

        input!(inputs.right, self.right_frames, actions, Right);

        if inputs.ccw {
            actions.push(Action::Ccw);
        }
        if inputs.hold {
            actions.push(Action::Hold);
        }
        if inputs.hard_drop {
            actions.push(Action::HardDrop)
        } else if inputs.soft_drop {
            actions.push(Action::SoftDrop);
        }
        input!(inputs.cw, self.cw, actions, Cw);
        input!(inputs.ccw, self.ccw, actions, Ccw);
        actions
    }
}

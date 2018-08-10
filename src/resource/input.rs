#![allow(non_snake_case)]

use std::collections::HashMap;

use winit::{KeyboardInput, VirtualKeyCode, ElementState};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InputState {
    Pressed, // Just been pressed
    Released, // Just been released
}

macro_rules! input {
    (len: $len:expr, [ $(($idx:expr) = $n:ident: $($keys:pat)|*),* ]) => {
        #[derive(Default)]
        pub struct InputList {
            pub inputs: [Option<InputState>; $len],
        }

        impl InputList {
            pub fn new() -> InputList {
                InputList {
                    inputs: [None; $len],
                }
            }

            fn input_to_index(inp: &Input) -> usize {
                match *inp {
                    $(
                        Input::$n => $idx,
                    )*
                }
            }

            pub fn input_state(&self, inp: Input) -> Option<InputState> {
                let idx = Self::input_to_index(&inp);
                
                self.inputs.get(idx).and_then(|x| *x)
            }

            pub fn set_input(&mut self, inp: Input, state: InputState) {
                let idx = Self::input_to_index(&inp);
                
                if let Some(elem) = self.inputs.get_mut(idx) {
                    *elem = Some(state);
                }
            }
        }

        #[derive(Debug)]
        pub enum Input {
            $($n,)*
        }

        pub fn key_to_input(key: &KeyboardInput) -> Option<(Input, InputState)> {
            let state = match key.state { 
                ElementState::Pressed => InputState::Pressed, 
                ElementState::Released => InputState::Released,
            };
            
            match *key {
                $(
                    $($keys)|* => Some((Input::$n, state)),
                )*
                _ => None,
            }
        }
    };
}

input!(
    len: 4,
    [
        (0) = Up: 
            KeyboardInput { virtual_keycode: Some(VirtualKeyCode::W), .. } 
            | KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Up), .. },
        (1) = Down: 
            KeyboardInput { virtual_keycode: Some(VirtualKeyCode::S), .. } 
            | KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Down), .. },
        (2) = Left: 
            KeyboardInput { virtual_keycode: Some(VirtualKeyCode::A), .. } 
            | KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Left), .. },
        (3) = Right: 
            KeyboardInput { virtual_keycode: Some(VirtualKeyCode::D), .. } 
            | KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Right), .. }
    ]
);
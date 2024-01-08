use std::convert::TryFrom;
use std::ffi::c_char;

use super::convert_key_code;
use super::input_remapping::ButtonInput;
use super::KeyEvent;
use super::RemapInputValue;

pub(crate) struct ButtonRemapper {
    key_code: c_char,
}

impl TryFrom<&ButtonInput> for ButtonRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &ButtonInput) -> Result<Self, Self::Error> {
        convert_key_code(input.key_code).map(|key_code| Self { key_code })
    }
}

impl RemapInputValue for ButtonRemapper {
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>> {
        Some(vec![KeyEvent {
            key_code: self.key_code,
            is_pressed: value != 0,
        }])
    }
}

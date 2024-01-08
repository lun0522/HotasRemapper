use std::convert::TryFrom;
use std::ffi::c_char;

use anyhow::anyhow;

use super::input_remapping::ButtonInput;
use super::KeyEvent;
use super::RemapInputValue;

pub(crate) struct ButtonRemapper {
    key_code: c_char,
}

impl TryFrom<&ButtonInput> for ButtonRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &ButtonInput) -> Result<Self, Self::Error> {
        match c_char::try_from(input.key_code) {
            Ok(key_code) => Ok(Self { key_code }),
            Err(e) => {
                Err(anyhow!("Cannot convert {} to char: {}", input.key_code, e))
            }
        }
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

use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use super::convert_key_code;
use super::KeyEvent;
use super::RemapInputValue;
use crate::input_remapping::ButtonInput;

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

impl Display for ButtonRemapper {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!("{}", self.key_code))
    }
}

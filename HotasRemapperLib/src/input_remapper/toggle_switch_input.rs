use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use super::convert_key_codes;
use super::KeyEvent;
use super::RemapInputValue;
use crate::input_remapping::ToggleSwitchInput;

pub(crate) struct ToggleSwitchRemapper {
    on_key_code: c_char,
    off_key_code: c_char,
}

impl TryFrom<&ToggleSwitchInput> for ToggleSwitchRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &ToggleSwitchInput) -> Result<Self, Self::Error> {
        convert_key_codes(&[input.on_key_code, input.off_key_code]).map(
            |key_codes| Self {
                on_key_code: key_codes[0],
                off_key_code: key_codes[1],
            },
        )
    }
}

impl RemapInputValue for ToggleSwitchRemapper {
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>> {
        let key_code = if value != 0 {
            self.on_key_code
        } else {
            self.off_key_code
        };
        Some(vec![
            KeyEvent {
                key_code,
                is_pressed: true,
            },
            KeyEvent {
                key_code,
                is_pressed: false,
            },
        ])
    }
}

impl Display for ToggleSwitchRemapper {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!(
            "{{on: {}, off: {}}}",
            self.on_key_code, self.off_key_code
        ))
    }
}

use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use super::convert_key_codes;
use super::RemapInputValue;
use crate::input_remapping::ToggleSwitchInput;
use crate::virtual_device::KeyEvent;

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
    fn remap(&self, value: i32) -> Option<KeyEvent> {
        let key_code = if value != 0 {
            self.on_key_code
        } else {
            self.off_key_code
        };
        Some(KeyEvent::PressAndRelease(key_code))
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

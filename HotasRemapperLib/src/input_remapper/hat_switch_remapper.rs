use std::cell::RefCell;
use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use anyhow::anyhow;

use super::convert_key_codes;
use super::RemapInputValue;
use crate::input_remapping::HatSwitchInput;
use crate::virtual_device::KeyEvent;

pub(crate) struct HatSwitchRemapper {
    key_codes: Vec<c_char>,
    last_key_code: RefCell<c_char>,
}

impl TryFrom<&HatSwitchInput> for HatSwitchRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &HatSwitchInput) -> Result<Self, Self::Error> {
        let num_key_codes = input.key_codes.len();
        if num_key_codes == 4 || num_key_codes == 8 {
            let key_codes = convert_key_codes(&input.key_codes)?;
            Ok(Self {
                key_codes,
                last_key_code: RefCell::new(0),
            })
        } else {
            Err(anyhow!(
                "Number of key codes ({}) provided is neither 4 or 8",
                num_key_codes
            ))
        }
    }
}

impl RemapInputValue for HatSwitchRemapper {
    fn remap(&self, value: i32) -> Option<KeyEvent> {
        // An 8-way switch may emit value 15 to signal that the hat has returned
        // to the center, so we can't always use `value` as the index.
        let curr_key_code =
            self.key_codes.get(value as usize).cloned().unwrap_or(0);
        if curr_key_code == *self.last_key_code.borrow() {
            return None;
        }
        let last_key_code = *self.last_key_code.borrow();
        self.last_key_code.replace(curr_key_code);
        Some(if last_key_code == 0 {
            KeyEvent::Press(curr_key_code)
        } else if curr_key_code == 0 {
            KeyEvent::Release(last_key_code)
        } else {
            KeyEvent::ReleaseAndPress {
                to_release: last_key_code,
                to_press: curr_key_code,
            }
        })
    }
}

impl Display for HatSwitchRemapper {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!("{:?}", self.key_codes))
    }
}

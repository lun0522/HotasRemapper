use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use super::convert_key_codes;
use super::RemapInputValue;
use crate::input_remapping::AxisInput;
use crate::virtual_device::KeyEvent;

pub(crate) struct AxisRemapper {
    key_codes: Vec<c_char>,
    min_value: f64,
    interval: f64,
}

impl TryFrom<&AxisInput> for AxisRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &AxisInput) -> Result<Self, Self::Error> {
        let key_codes = convert_key_codes(&input.key_codes)?;
        let (min_value, max_value) = if !input.reverse_axis {
            (input.min_value as f64, input.max_value as f64)
        } else {
            (input.max_value as f64, input.min_value as f64)
        };
        let interval = (max_value - min_value) / (key_codes.len() - 1) as f64;
        Ok(Self {
            key_codes,
            min_value,
            interval,
        })
    }
}

impl RemapInputValue for AxisRemapper {
    fn remap(&mut self, value: i32) -> Option<KeyEvent> {
        let index_f64 = (value as f64 - self.min_value) / self.interval;
        let index =
            (index_f64.round() as usize).clamp(0, self.key_codes.len() - 1);
        let key_code = self.key_codes[index];
        Some(KeyEvent::PressAndRelease(key_code))
    }
}

impl Display for AxisRemapper {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!("{:?}", self.key_codes))
    }
}

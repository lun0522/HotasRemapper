use std::convert::TryFrom;
use std::ffi::c_char;

use super::convert_key_codes;
use super::input_remapping::AxisInput;
use super::KeyEvent;
use super::RemapInputValue;

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
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>> {
        let index_f64 = (value as f64 - self.min_value) / self.interval;
        let index =
            (index_f64.round() as usize).clamp(0, self.key_codes.len() - 1);
        let key_code = self.key_codes[index];
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

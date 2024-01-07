use std::collections::HashMap;
use std::convert::From;
use std::convert::TryFrom;
use std::ffi::c_char;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use core::result::Result as CoreResult;
use input_remapping::AxisInput;
use input_remapping::ButtonInput;
use input_remapping::InputRemapping;
use protobuf::text_format::parse_from_str as parse_proto_from_str;

use crate::hid_device::DeviceType;
use crate::hid_device::InputEvent;
use crate::hid_device_input::DeviceInput;
use crate::hid_device_input::InputType;

pub(crate) struct KeyEvent {
    pub key_code: c_char,
    pub is_pressed: bool,
}

#[derive(Eq, Hash, PartialEq)]
struct InputIdentifier {
    pub device_type: DeviceType,
    pub device_input: DeviceInput,
}

impl From<&InputEvent> for InputIdentifier {
    fn from(event: &InputEvent) -> Self {
        Self {
            device_type: event.device_type,
            device_input: event.device_input,
        }
    }
}

trait RemapInputValue {
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>>;
}

pub(crate) struct InputRemapper {
    input_remapping: HashMap<InputIdentifier, Box<dyn RemapInputValue>>,
}

impl InputRemapper {
    pub fn new() -> Self {
        Self {
            input_remapping: Default::default(),
        }
    }

    pub fn load_remapping_from_file(&mut self, file_path: &str) -> Result<()> {
        self.input_remapping.clear();
        let file_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => bail!("Failed to read file: {}", e),
        };
        let input_remapping =
            match parse_proto_from_str::<InputRemapping>(&file_content) {
                Ok(remapping) => remapping,
                Err(e) => bail!("Failed to parse as text proto: {}", e),
            };
        let throttle_button_remapping =
            match input_remapping.throttle_inputs.get("button") {
                Some(remapping) => remapping,
                None => bail!("Throttle remapping not found!"),
            };
        for (index, input) in throttle_button_remapping.inputs.iter() {
            if !input.has_button_input() {
                continue;
            }
            println!(
                "Remapping throttle button {} to {}",
                index,
                input.button_input().key_code
            );
            self.input_remapping.insert(
                InputIdentifier {
                    device_type: DeviceType::Throttle,
                    device_input: DeviceInput {
                        input_type: InputType::Button,
                        index: *index,
                    },
                },
                Box::new(ButtonRemapper::try_from(input.button_input())?),
            );
        }
        let throttle_z_axis_remapping =
            match input_remapping.throttle_inputs.get("z_axis") {
                Some(remapping) => remapping,
                None => bail!("Throttle z-axis not found!"),
            };
        for (index, input) in throttle_z_axis_remapping.inputs.iter() {
            if !input.has_axis_input() {
                continue;
            }
            println!(
                "Remapping throttle z-axis {} to {:?}",
                index,
                input.axis_input().key_codes
            );
            self.input_remapping.insert(
                InputIdentifier {
                    device_type: DeviceType::Throttle,
                    device_input: DeviceInput {
                        input_type: InputType::ZAxis,
                        index: *index,
                    },
                },
                Box::new(AxisRemapper::try_from(input.axis_input())?),
            );
        }
        Ok(())
    }

    pub fn remap_input_event(
        &mut self,
        input_event: &InputEvent,
    ) -> Option<Vec<KeyEvent>> {
        self.input_remapping
            .get_mut(&input_event.into())
            .and_then(|remapper| remapper.remap(input_event.value))
    }
}

struct ButtonRemapper {
    key_code: c_char,
}

impl TryFrom<&ButtonInput> for ButtonRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &ButtonInput) -> CoreResult<Self, Self::Error> {
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

struct AxisRemapper {
    key_codes: Vec<c_char>,
    min_value: f64,
    interval: f64,
    // If the axis input value stays in the same range as before, we don't have
    // to emit any key events, so we keep track of the latest used key code.
    prev_key_code: Option<c_char>,
}

impl TryFrom<&AxisInput> for AxisRemapper {
    type Error = anyhow::Error;

    fn try_from(input: &AxisInput) -> CoreResult<Self, Self::Error> {
        let key_codes = input
            .key_codes
            .iter()
            .map(|key_code_i32| match c_char::try_from(*key_code_i32) {
                Ok(key_code) => Ok(key_code),
                Err(e) => Err(anyhow!(
                    "Cannot convert {} to char: {}",
                    key_code_i32,
                    e
                )),
            })
            .collect::<CoreResult<Vec<_>, _>>()?;
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
            prev_key_code: None,
        })
    }
}

impl RemapInputValue for AxisRemapper {
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>> {
        let index_f64 = (value as f64 - self.min_value) / self.interval;
        let index =
            (index_f64.round() as usize).clamp(0, self.key_codes.len() - 1);
        let key_code = self.key_codes[index];
        if self
            .prev_key_code
            .map(|prev_key_code| prev_key_code == key_code)
            .unwrap_or_default()
        {
            return None;
        }

        self.prev_key_code = Some(key_code);
        Some(vec![
            KeyEvent {
                key_code,
                is_pressed: true,
            },
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

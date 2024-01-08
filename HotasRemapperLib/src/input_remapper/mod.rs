include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

mod axis_remapper;
mod button_remapper;
mod hat_switch_remapper;

use std::collections::HashMap;
use std::convert::From;
use std::ffi::c_char;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use anyhow::Result;
use axis_remapper::AxisRemapper;
use button_remapper::ButtonRemapper;
use core::result::Result as CoreResult;
use hat_switch_remapper::HatSwitchRemapper;
use input_remapping::InputRemapping;
use protobuf::text_format::parse_from_str as parse_proto_from_str;

use crate::input_reader::hid_device::DeviceType;
use crate::input_reader::hid_device::InputEvent;
use crate::input_reader::hid_device_input::DeviceInput;
use crate::input_reader::hid_device_input::InputType;

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
        let joystick_hat_switch_remapping =
            match input_remapping.joystick_inputs.get("hat_switch") {
                Some(remapping) => remapping,
                None => bail!("Joystick hat switch not found!"),
            };
        for (index, input) in joystick_hat_switch_remapping.inputs.iter() {
            if !input.has_hat_switch_input() {
                continue;
            }
            println!(
                "Remapping joystick hat switch {} to {:?}",
                index,
                input.hat_switch_input().key_codes
            );
            self.input_remapping.insert(
                InputIdentifier {
                    device_type: DeviceType::Joystick,
                    device_input: DeviceInput {
                        input_type: InputType::Hat,
                        index: *index,
                    },
                },
                Box::new(HatSwitchRemapper::try_from(
                    input.hat_switch_input(),
                )?),
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

fn convert_key_code(key_code: i32) -> CoreResult<c_char, Error> {
    c_char::try_from(key_code)
        .map_err(|e| anyhow!("Cannot convert {} to char: {}", key_code, e))
}

fn convert_key_codes(key_codes: &Vec<i32>) -> CoreResult<Vec<c_char>, Error> {
    key_codes.iter().cloned().map(convert_key_code).collect()
}

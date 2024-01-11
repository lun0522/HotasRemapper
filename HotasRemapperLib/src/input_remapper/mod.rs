mod axis_remapper;
mod button_remapper;
mod hat_switch_remapper;

use std::collections::HashMap;
use std::convert::From;
use std::convert::TryFrom;
use std::ffi::c_char;
use std::fmt::Display;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use anyhow::Result;
use axis_remapper::AxisRemapper;
use button_remapper::ButtonRemapper;
use core::result::Result as CoreResult;
use hat_switch_remapper::HatSwitchRemapper;
use protobuf::text_format::parse_from_str as parse_proto_from_str;

use crate::input_reader::hid_device::DeviceType;
use crate::input_reader::hid_device::InputEvent;
use crate::input_reader::hid_device_input::DeviceInput;
use crate::input_reader::hid_device_input::InputType;
use crate::input_remapping::InputRemapping;
use crate::input_remapping::RemappedInput;

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

trait RemapInputValue: Display {
    fn remap(&mut self, value: i32) -> Option<Vec<KeyEvent>>;
}

pub(crate) struct InputRemapper {
    input_remappers: HashMap<InputIdentifier, Box<dyn RemapInputValue>>,
}

impl InputRemapper {
    pub fn new() -> Self {
        Self {
            input_remappers: Default::default(),
        }
    }

    pub fn load_input_remapping(
        &mut self,
        encoded_input_remapping: &str,
    ) -> Result<()> {
        self.input_remappers.clear();
        let input_remapping =
            parse_proto_from_str::<InputRemapping>(&encoded_input_remapping)
                .map_err(|e| anyhow!("Failed to parse as text proto: {}", e))?;
        self.load_remapping_for_device(&input_remapping, DeviceType::Joystick)?;
        self.load_remapping_for_device(&input_remapping, DeviceType::Throttle)?;
        Ok(())
    }

    pub fn remap_input_event(
        &mut self,
        input_event: &InputEvent,
    ) -> Option<Vec<KeyEvent>> {
        self.input_remappers
            .get_mut(&input_event.into())
            .and_then(|remapper| remapper.remap(input_event.value))
    }

    fn load_remapping_for_device(
        &mut self,
        input_remapping: &InputRemapping,
        device_type: DeviceType,
    ) -> Result<()> {
        let remapped_inputs = match device_type {
            DeviceType::Joystick => &input_remapping.joystick_inputs,
            DeviceType::Throttle => &input_remapping.throttle_inputs,
        };
        for (input_type_name, inputs) in remapped_inputs.iter() {
            let input_type: InputType = match input_type_name
                .as_str()
                .try_into()
            {
                Ok(input_type) => input_type,
                Err(_) => bail!("Unknown input type name: {}", input_type_name),
            };
            for (index, input) in inputs.inputs.iter() {
                let device_input = DeviceInput {
                    input_type,
                    index: *index,
                };
                let input_remapper = Self::create_input_remapper(input)?;
                println!(
                    "Remapping {:?} {} to {}",
                    device_type, device_input, input_remapper
                );
                self.input_remappers.insert(
                    InputIdentifier {
                        device_type,
                        device_input,
                    },
                    input_remapper,
                );
            }
        }
        Ok(())
    }

    fn create_input_remapper(
        input: &RemappedInput,
    ) -> Result<Box<dyn RemapInputValue>> {
        Ok(if input.has_button_input() {
            Box::new(ButtonRemapper::try_from(input.button_input())?)
        } else if input.has_toggle_switch_input() {
            unimplemented!()
        } else if input.has_hat_switch_input() {
            Box::new(HatSwitchRemapper::try_from(input.hat_switch_input())?)
        } else if input.has_axis_input() {
            Box::new(AxisRemapper::try_from(input.axis_input())?)
        } else {
            unreachable!()
        })
    }
}

impl TryFrom<&str> for InputType {
    type Error = &'static str;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "button" => Ok(InputType::Button),
            "hat" => Ok(InputType::Hat),
            "x-axis" => Ok(InputType::XAxis),
            "y-axis" => Ok(InputType::YAxis),
            "z-axis" => Ok(InputType::ZAxis),
            "rx-axis" => Ok(InputType::RxAxis),
            "ry-axis" => Ok(InputType::RyAxis),
            "rz-axis" => Ok(InputType::RzAxis),
            "slider" => Ok(InputType::Slider),
            _ => Err("Unknown type"),
        }
    }
}

fn convert_key_code(key_code: i32) -> CoreResult<c_char, Error> {
    c_char::try_from(key_code)
        .map_err(|e| anyhow!("Cannot convert {} to char: {}", key_code, e))
}

fn convert_key_codes(key_codes: &Vec<i32>) -> CoreResult<Vec<c_char>, Error> {
    key_codes.iter().cloned().map(convert_key_code).collect()
}

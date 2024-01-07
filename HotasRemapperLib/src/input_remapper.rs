use std::collections::HashMap;
use std::convert::From;
use std::ffi::c_char;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use anyhow::bail;
use anyhow::Result;
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

pub(crate) struct InputRemapper {
    input_remapping: HashMap<InputIdentifier, c_char>,
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
                input.button_input().key_code as i8,
            );
        }
        Ok(())
    }

    pub fn remap_input_event(
        &self,
        input_event: &InputEvent,
    ) -> Option<KeyEvent> {
        self.input_remapping
            .get(&input_event.into())
            .map(|mapped_key| KeyEvent {
                key_code: *mapped_key,
                is_pressed: input_event.value != 0,
            })
    }
}

use std::collections::HashMap;
use std::convert::From;
use std::ffi::c_char;

use anyhow::bail;
use anyhow::Result;

use crate::hid_device::DeviceType;
use crate::hid_device::InputEvent;
use crate::hid_device_input::DeviceInput;
use crate::hid_device_input::InputType;

type JsonValue = serde_json::Value;

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
    input_mapping: HashMap<InputIdentifier, c_char>,
}

impl InputRemapper {
    pub fn new() -> Self {
        Self {
            input_mapping: Default::default(),
        }
    }

    pub fn load_mapping_from_file(&mut self, file_path: &str) -> Result<()> {
        self.input_mapping.clear();
        let file_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => bail!("Failed to read file: {}", e),
        };
        let all_mapping = match serde_json::from_str(&file_content) {
            Ok(mapping_json) => match mapping_json {
                JsonValue::Object(mapping) => mapping,
                _ => bail!("File content is not a map: {}", mapping_json),
            },
            Err(e) => bail!("Failed to parse as JSON: {}", e),
        };
        let throttle_mapping = match all_mapping.get("throttle") {
            Some(mapping_json) => match mapping_json {
                JsonValue::Object(mapping) => mapping,
                _ => bail!("Throttle field is not a map: {}", mapping_json),
            },
            None => bail!("Throttle field not found!"),
        };
        let button_mapping = match throttle_mapping.get("button") {
            Some(button_mapping_json) => match button_mapping_json {
                JsonValue::Object(mapping) => mapping,
                _ => bail!(
                    "Throttle button field is not a map: {}",
                    button_mapping_json
                ),
            },
            None => bail!("Throttle button field not found!"),
        };
        for (key, value) in button_mapping.iter() {
            let button_index = match key.parse::<i32>() {
                Ok(index) => index,
                Err(e) => bail!("Cannot parse {} as int: {}", key, e),
            };
            let mapped_key = match value {
                JsonValue::Number(number) => match number.as_i64() {
                    Some(int_number) => int_number as c_char,
                    None => bail!("{} is not an integer", number),
                },
                _ => bail!("{} is not a number", value),
            };
            println!(
                "Remapping throttle button {} to {}",
                button_index, mapped_key
            );
            self.input_mapping.insert(
                InputIdentifier {
                    device_type: DeviceType::Throttle,
                    device_input: DeviceInput {
                        input_type: InputType::Button,
                        index: button_index,
                    },
                },
                mapped_key,
            );
        }
        Ok(())
    }

    pub fn remap_input_event(
        &self,
        input_event: &InputEvent,
    ) -> Option<KeyEvent> {
        self.input_mapping
            .get(&input_event.into())
            .map(|mapped_key| KeyEvent {
                key_code: *mapped_key,
                is_pressed: input_event.value != 0,
            })
    }
}

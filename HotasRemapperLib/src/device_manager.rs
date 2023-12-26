use std::convert::TryFrom;

use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueRef;

use crate::hid_device::DeviceId;
use crate::hid_device::HIDDevice;
use crate::hid_device::HandleInputValue;

#[derive(Debug)]
enum DeviceType {
    Joystick,
    Throttle,
}

impl TryFrom<&DeviceId> for DeviceType {
    type Error = &'static str;

    fn try_from(device_id: &DeviceId) -> Result<Self, Self::Error> {
        match device_id.device_name.as_str() {
            "Throttle - HOTAS Warthog" => Ok(Self::Joystick),
            "Joystick - HOTAS Warthog" => Ok(Self::Throttle),
            _ => Err("Unrecognized"),
        }
    }
}

pub(crate) struct DeviceManager {
    joystick_device: Option<HIDDevice>,
    throttle_device: Option<HIDDevice>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            joystick_device: None,
            throttle_device: None,
        }
    }

    pub fn handle_device_matched(
        &mut self,
        device: IOHIDDeviceRef,
        input_value_handler: &dyn HandleInputValue,
    ) {
        // Safe because the device is alive.
        let open_device = |device_id: DeviceId| unsafe {
            HIDDevice::open_device(device_id, input_value_handler)
        };
        let device_id = DeviceId::from_device(device);
        if let Some(device_type) = DeviceType::try_from(&device_id).ok() {
            match device_type {
                DeviceType::Joystick => {
                    if self.joystick_device.is_none() {
                        println!("Found Joystick device: {}", device_id);
                        // Safe because the device is alive.
                        self.joystick_device = Some(open_device(device_id));
                        return;
                    }
                }
                DeviceType::Throttle => {
                    if self.throttle_device.is_none() {
                        println!("Found Throttle device: {}", device_id);
                        // Safe because the device is alive.
                        self.throttle_device = Some(open_device(device_id));
                        return;
                    }
                }
            }
        }
        println!("Ignoring device: {}", device_id);
    }

    pub fn handle_device_removed(&mut self, device: IOHIDDeviceRef) {
        println!("Device removed: {}", DeviceId::from_device(device));
    }

    pub fn handle_input_value(&mut self, value: IOHIDValueRef) {
        if let Some(input_event) = HIDDevice::read_input_event(value) {
            if let Some(device) = self.joystick_device.as_ref() {
                if device.device_ref() == input_event.device_ref {
                    device.handle_input_event(input_event);
                    return;
                }
            }
            if let Some(device) = self.throttle_device.as_ref() {
                if device.device_ref() == input_event.device_ref {
                    device.handle_input_event(input_event);
                    return;
                }
            }
        }
    }
}

use std::collections::HashMap;
use std::convert::TryFrom;

use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueRef;

use crate::hid_device::DeviceProperty;
use crate::hid_device::DeviceType;
use crate::hid_device::HIDDevice;
use crate::hid_device::HandleInputValue;
use crate::hid_manager::HandleDeviceEvent;

pub(crate) struct DeviceManager {
    devices: HashMap<IOHIDDeviceRef, HIDDevice>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Default::default(),
        }
    }
}

impl HandleDeviceEvent for DeviceManager {
    fn handle_device_matched(
        &mut self,
        device_ref: IOHIDDeviceRef,
        input_value_handler: &dyn HandleInputValue,
    ) {
        let device_property = DeviceProperty::from_device(device_ref);
        if let Some(device_type) = DeviceType::try_from(&device_property).ok() {
            // Open a new device only if we haven't found any devices of the
            // same type.
            if !self
                .devices
                .iter()
                .any(|(_, device)| device.device_type() == device_type)
            {
                println!("Found {:?} device: {}", device_type, device_property);
                // Safe because the device is alive.
                self.devices.insert(device_ref, unsafe {
                    HIDDevice::open_device(
                        device_ref,
                        device_type,
                        input_value_handler,
                    )
                });
                return;
            }
        }
        println!("Ignoring device: {}", device_property);
    }

    fn handle_device_removed(&mut self, device_ref: IOHIDDeviceRef) {
        if let Some(device) = self.devices.remove(&device_ref) {
            println!("Removed {:?} device", device.device_type());
        }
    }

    fn handle_input_value(&mut self, value: IOHIDValueRef) {
        if let Some(input_event) = HIDDevice::read_input_event(value) {
            if let Some(device) = self.devices.get(&input_event.device_ref) {
                device.handle_input_event(input_event);
                return;
            }
        }
    }
}

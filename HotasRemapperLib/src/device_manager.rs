use io_kit_sys::hid::base::IOHIDDeviceRef;

use crate::hid_device::DeviceId;

pub(crate) struct DeviceManager {}

impl DeviceManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_device_matched(&mut self, device: IOHIDDeviceRef) {
        println!("Found matching device: {}", DeviceId::from_device(device));
    }

    pub fn handle_device_removed(&mut self, device: IOHIDDeviceRef) {
        println!("Device removed: {}", DeviceId::from_device(device));
    }
}

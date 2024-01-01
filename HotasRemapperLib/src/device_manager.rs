use std::collections::HashMap;
use std::convert::From;
use std::convert::TryFrom;

use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueRef;

use crate::hid_device::DeviceProperty;
use crate::hid_device::HIDDevice;
use crate::hid_device::HandleInputValue;
use crate::hid_manager::HandleDeviceEvent;
use crate::ConnectionStatusCallback;
use crate::DeviceType;

type HIDDeviceType = crate::hid_device::DeviceType;

pub(crate) struct DeviceManager {
    devices: HashMap<IOHIDDeviceRef, HIDDevice>,
    device_types: HashMap<HIDDeviceType, IOHIDDeviceRef>,
    connection_status_callback: ConnectionStatusCallback,
}

impl DeviceManager {
    pub fn new(connection_status_callback: ConnectionStatusCallback) -> Self {
        Self {
            devices: Default::default(),
            device_types: Default::default(),
            connection_status_callback,
        }
    }

    fn report_connection_status(
        &self,
        device_type: HIDDeviceType,
        is_connected: bool,
    ) {
        // Safe because the caller guarantees the callback remains a valid
        // function pointer.
        unsafe {
            (self.connection_status_callback)(device_type.into(), is_connected)
        };
    }
}

impl HandleDeviceEvent for DeviceManager {
    fn handle_device_matched(
        &mut self,
        device_ref: IOHIDDeviceRef,
        input_value_handler: &dyn HandleInputValue,
    ) {
        let device_property = DeviceProperty::from_device(device_ref);
        if let Some(device_type) =
            HIDDeviceType::try_from(&device_property).ok()
        {
            // Open a new device only if we haven't found any devices of the
            // same type.
            if self.device_types.get(&device_type).is_none() {
                println!("Found {:?} device: {}", device_type, device_property);
                // Safe because the device is alive.
                self.devices.insert(device_ref, unsafe {
                    HIDDevice::open_device(
                        device_ref,
                        device_type,
                        input_value_handler,
                    )
                });
                self.device_types.insert(device_type, device_ref);
                self.report_connection_status(
                    device_type,
                    /* is_connected= */ true,
                );
                return;
            }
        }
        println!("Ignoring device: {}", device_property);
    }

    fn handle_device_removed(&mut self, device_ref: IOHIDDeviceRef) {
        if let Some(device) = self.devices.remove(&device_ref) {
            let device_type = device.device_type();
            self.device_types
                .remove(&device_type)
                .expect("Device not found in device_types map");
            self.report_connection_status(
                device_type,
                /* is_connected= */ false,
            );
            println!("Removed {:?} device", device_type);
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

impl From<HIDDeviceType> for DeviceType {
    fn from(value: HIDDeviceType) -> Self {
        match value {
            HIDDeviceType::Joystick => DeviceType::Joystick,
            HIDDeviceType::Throttle => DeviceType::Throttle,
        }
    }
}

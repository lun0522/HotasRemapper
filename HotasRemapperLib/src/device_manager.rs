use std::collections::HashMap;
use std::convert::From;
use std::convert::TryFrom;
use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;

use anyhow::Result;
use io_kit_sys::hid::base::IOHIDDeviceCallback;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueCallback;
use io_kit_sys::hid::base::IOHIDValueRef;
use io_kit_sys::ret::IOReturn;

use crate::hid_device::DeviceProperty;
use crate::hid_device::HIDDevice;
use crate::hid_device::HandleInputEvent;
use crate::hid_manager::HIDManager;
use crate::hid_manager::HandleDeviceEvent;
use crate::ConnectionStatusCallback;
use crate::DeviceType;

type HIDDeviceType = crate::hid_device::DeviceType;

pub(crate) struct DeviceManager {
    hid_manager: HIDManager,
    hid_devices: HashMap<IOHIDDeviceRef, HIDDevice>,
    connection_status_callback: ConnectionStatusCallback,
    // We want to make sure the `DeviceManager` doesn't get moved, so the user
    // can rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl DeviceManager {
    pub fn new(
        connection_status_callback: ConnectionStatusCallback,
    ) -> Result<Pin<Box<Self>>> {
        let mut manager = Box::pin(Self {
            hid_manager: HIDManager::new()?,
            hid_devices: Default::default(),
            connection_status_callback,
            _pinned_marker: PhantomPinned,
        });
        // Safe because we won't move `DeviceManager` out of the pinned object,
        // and it lives longer than its member `HIDManager`.
        unsafe {
            let pinned_manager_ptr =
                manager.as_mut().get_unchecked_mut() as *mut Self as *mut _;
            manager
                .as_ref()
                .hid_manager
                .set_device_callbacks(pinned_manager_ptr);
        }
        Ok(manager)
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

    fn handle_device_matched(&mut self, device_ref: IOHIDDeviceRef) {
        let device_property = DeviceProperty::from_device(device_ref);
        if let Some(device_type) =
            HIDDeviceType::try_from(&device_property).ok()
        {
            // Open a new device only if we haven't found any devices of the
            // same type.
            if !self
                .hid_devices
                .iter()
                .any(|(_, device)| device.device_type() == device_type)
            {
                println!("Found {:?} device: {}", device_type, device_property);
                let pinned_manager_ptr = self as *mut DeviceManager;
                // Safe because the device is alive, and `self` lives longer
                // than it.
                self.hid_devices.insert(device_ref, unsafe {
                    HIDDevice::open_device(
                        device_ref,
                        device_type,
                        pinned_manager_ptr,
                    )
                });
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
        if let Some(device) = self.hid_devices.remove(&device_ref) {
            let device_type = device.device_type();
            self.report_connection_status(
                device_type,
                /* is_connected= */ false,
            );
            println!("Removed {:?} device", device_type);
        }
    }

    fn handle_input_received(&mut self, value: IOHIDValueRef) {
        if let Some(input_event) = HIDDevice::read_input_event(value) {
            if let Some(device) = self.hid_devices.get(&input_event.device_ref)
            {
                device.handle_input_event(input_event);
                return;
            }
        }
    }
}

impl HandleDeviceEvent for DeviceManager {
    fn device_matched_callback() -> IOHIDDeviceCallback {
        handle_device_matched
    }

    fn device_removed_callback() -> IOHIDDeviceCallback {
        handle_device_removed
    }
}

impl HandleInputEvent for DeviceManager {
    fn input_received_callback() -> IOHIDValueCallback {
        handle_input_received
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

extern "C" fn handle_device_matched(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_device_matched(device);
    }
}

extern "C" fn handle_device_removed(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_device_removed(device);
    }
}

extern "C" fn handle_input_received(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDValueRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_input_received(value);
    }
}

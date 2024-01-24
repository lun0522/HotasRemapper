use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use anyhow::Result;
use objc::msg_send;
use objc::runtime::Object;
use objc::runtime::Sel;
use objc::sel;
use objc::sel_impl;

use crate::utils::new_string_from_ptr;

pub(crate) struct DeviceInfo {
    pub name: String,
    pub mac_address: String,
}

impl DeviceInfo {
    pub fn new(device: *const Object) -> Self {
        Self {
            name: unsafe { Self::get_name(device) },
            mac_address: unsafe { Self::get_mac_address(device) },
        }
    }

    unsafe fn get_name(device: *const Object) -> String {
        let name: *const Object = unsafe { msg_send![device, name] };
        unsafe { new_string_from_nsstring(name) }
            .unwrap_or("Unknown name".to_string())
    }

    unsafe fn get_mac_address(device: *const Object) -> String {
        let mac_address: *const Object =
            unsafe { msg_send![device, addressString] };
        unsafe { new_string_from_nsstring(mac_address) }
            .unwrap_or("Unknown MAC address".to_string())
    }
}

impl Display for DeviceInfo {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!(
            "{{device name: {:?}, MAC address: {}}}",
            self.name, self.mac_address
        ))
    }
}

pub(crate) struct DeviceEventHandler {
    pub handler_ptr: *const Object,
    pub on_device_disconnected: Sel,
}

pub(crate) struct BluetoothDevice {}

impl BluetoothDevice {
    pub fn new(device: *const Object, handler: DeviceEventHandler) -> Self {
        unsafe {
            Self::register_for_device_disconnection_notification(
                device,
                handler.handler_ptr,
                handler.on_device_disconnected,
            )
        };
        Self {}
    }

    unsafe fn register_for_device_disconnection_notification(
        device: *const Object,
        handler_ptr: *const Object,
        on_device_disconnected: Sel,
    ) {
        let () = msg_send![
            device,
            registerForDisconnectNotification: handler_ptr
            selector: on_device_disconnected];
    }
}

unsafe fn new_string_from_nsstring(nsstring: *const Object) -> Result<String> {
    let string_ptr: *const c_char = msg_send![nsstring, UTF8String];
    unsafe { new_string_from_ptr(string_ptr) }
}

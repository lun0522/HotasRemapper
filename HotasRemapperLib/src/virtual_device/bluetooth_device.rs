use std::ffi::c_char;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ptr::null_mut;

use anyhow::bail;
use anyhow::Result;
use io_kit_sys::ret::kIOReturnSuccess;
use io_kit_sys::ret::IOReturn;
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

pub(crate) struct BluetoothDevice {
    rfcomm_channel: *const Object,
    is_rfcomm_channel_opened: bool,
}

impl BluetoothDevice {
    /// `event_handler` must have implemented:
    ///   - `on_device_disconnected_selector()`
    ///   - `on_rfcomm_channel_closed_selector()`
    ///   - `on_rfcomm_channel_opened_selector()`
    pub fn new(
        device: *const Object,
        rfcomm_channel_id: u8,
        event_handler: *const Object,
    ) -> Result<Self> {
        unsafe {
            Self::register_for_device_disconnection_notification(
                device,
                event_handler,
            )
        };
        let rfcomm_channel = unsafe {
            Self::open_rfcomm_channel_async(
                device,
                rfcomm_channel_id,
                event_handler,
            )?
        };
        Ok(Self {
            rfcomm_channel,
            is_rfcomm_channel_opened: false,
        })
    }

    pub fn update_rfcomm_channel_status(&mut self, is_opened: bool) {
        self.is_rfcomm_channel_opened = is_opened;
    }

    pub fn send_data(&self, data: &[c_char]) {
        if !self.is_rfcomm_channel_opened {
            return;
        }
        let ret: IOReturn = unsafe {
            msg_send![
                self.rfcomm_channel,
                writeSync: data
                length: data.len() as u16]
        };
        if ret != kIOReturnSuccess {
            println!("Failed to write to RFCOMM channel: {}", ret);
        }
    }

    unsafe fn register_for_device_disconnection_notification(
        device: *const Object,
        event_handler: *const Object,
    ) {
        let () = msg_send![
            device,
            registerForDisconnectNotification: event_handler
            selector: on_device_disconnected_selector()];
    }

    unsafe fn open_rfcomm_channel_async(
        device: *const Object,
        rfcomm_channel_id: u8,
        event_handler: *const Object,
    ) -> Result<*const Object> {
        println!("Opening RFCOMM channel");
        let rfcomm_channel: *const Object = null_mut();
        let ret: IOReturn = msg_send![
            device,
            openRFCOMMChannelAsync: &rfcomm_channel
            withChannelID: rfcomm_channel_id
            delegate: event_handler];
        if ret != kIOReturnSuccess {
            bail!("Failed to open RFCOMM channel: {}", ret);
        }
        Ok(rfcomm_channel)
    }
}

impl Drop for BluetoothDevice {
    fn drop(&mut self) {
        if self.is_rfcomm_channel_opened {
            println!("Closing RFCOMM channel");
            unsafe {
                let () = msg_send![self.rfcomm_channel, close];
            }
        }
    }
}

unsafe fn new_string_from_nsstring(nsstring: *const Object) -> Result<String> {
    let string_ptr: *const c_char = msg_send![nsstring, UTF8String];
    unsafe { new_string_from_ptr(string_ptr) }
}

#[inline]
pub(crate) fn on_device_connected_selector() -> Sel {
    sel!(didConnectWithNotification:fromDevice:)
}

#[inline]
pub(crate) fn on_device_disconnected_selector() -> Sel {
    sel!(didDisconnectWithNotification:fromDevice:)
}

#[inline]
pub(crate) fn on_rfcomm_channel_opened_selector() -> Sel {
    sel!(rfcommChannelOpenComplete:status:)
}

#[inline]
pub(crate) fn on_rfcomm_channel_closed_selector() -> Sel {
    sel!(rfcommChannelClosed:)
}

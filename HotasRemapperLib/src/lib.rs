// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

mod device_manager;
mod hid_device;
mod hid_device_input;
mod hid_manager;
pub(crate) mod utils;

use std::ffi::c_void;

use device_manager::DeviceManager;
use swift_rs::swift;
use swift_rs::SwiftArg;

#[repr(C)]
pub enum DeviceType {
    Joystick = 0,
    Throttle = 1,
    VirtualDevice = 2,
}

type HIDManager = hid_manager::HIDManager<DeviceManager>;
pub(crate) type ConnectionStatusCallback =
    unsafe extern "C" fn(DeviceType, bool);
pub(crate) type VirtualDeviceConnectionStatusCallback =
    unsafe extern "C" fn(bool);

pub struct BluetoothLibCallback(pub VirtualDeviceConnectionStatusCallback);

impl<'a> SwiftArg<'a> for BluetoothLibCallback {
    type ArgType = VirtualDeviceConnectionStatusCallback;

    unsafe fn as_arg(&'a self) -> Self::ArgType {
        self.0
    }
}

swift!(fn OpenBluetoothLib(callback: BluetoothLibCallback));
swift!(fn CloseBluetoothLib());

static mut CONNECTION_STATUS_CALLBACK: Option<ConnectionStatusCallback> = None;

/// The caller must call `CloseLib()` at the end with the pointer returned by
/// `OpenLib()`, and `connection_status_callback()` must remain a valid function
/// pointer until then.
#[no_mangle]
pub unsafe extern "C" fn OpenLib(
    connection_status_callback: ConnectionStatusCallback,
) -> *mut c_void {
    println!("Opening {}", project_name());
    CONNECTION_STATUS_CALLBACK = Some(connection_status_callback);
    // Safe because we are just passing in a static function.
    unsafe {
        OpenBluetoothLib(BluetoothLibCallback(
            UpdateVirtualDeviceConnectionStatus,
        ))
    };
    match HIDManager::new(DeviceManager::new(connection_status_callback)) {
        Ok(mut manager) => {
            let manager_ptr = &*manager.as_mut() as *const HIDManager as *mut _;
            // We rely on the caller to call `CloseLib()` at the end to release
            // the pinned `HIDManager`.
            std::mem::forget(manager);
            manager_ptr
        }
        Err(e) => {
            println!("Failed to create HID manager: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// The caller must pass in the pointer returned by `OpenLib()`.
#[no_mangle]
pub unsafe extern "C" fn CloseLib(manager_ptr: *mut c_void) {
    println!("Closing {}", project_name());
    // Trivially safe.
    unsafe { CloseBluetoothLib() };
    if !manager_ptr.is_null() {
        std::ptr::drop_in_place(manager_ptr as *mut HIDManager);
    }
}

#[no_mangle]
pub unsafe extern "C" fn UpdateVirtualDeviceConnectionStatus(
    is_connected: bool,
) {
    if let Some(callback) = CONNECTION_STATUS_CALLBACK {
        callback(DeviceType::VirtualDevice, is_connected);
    }
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

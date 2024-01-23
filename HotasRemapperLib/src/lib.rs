// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

mod bluetooth_manager;
mod device_manager;
mod input_reader;
mod input_remapper;
pub(crate) mod utils;
mod virtual_device;

use std::ffi::c_char;
use std::ffi::c_void;

use anyhow::bail;
use anyhow::Result;
use device_manager::DeviceManager;

#[repr(C)]
pub enum ConnectionType {
    Joystick = 0,
    Throttle = 1,
    VirtualDevice = 2,
    RFCOMMChannel = 3,
}

pub(crate) type ConnectionStatusCallback =
    unsafe extern "C" fn(ConnectionType, bool);

/// The caller must call `CloseLib()` at the end with the pointer returned by
/// `OpenLib()`, and `connection_status_callback` must remain a valid function
/// pointer until then. Besides, `settings_ptr` must point to a UTF-8 encoded
/// `Settings` message.
#[no_mangle]
pub unsafe extern "C" fn OpenLib(
    settings_ptr: *const c_char,
    connection_status_callback: ConnectionStatusCallback,
) -> *mut c_void {
    println!("Opening {}", project_name());
    match DeviceManager::new(settings_ptr, connection_status_callback) {
        Ok(mut manager) => {
            let manager_ptr =
                &*manager.as_mut() as *const DeviceManager as *mut _;
            // We rely on the caller to call `CloseLib()` at the end to release
            // the pinned `DeviceManager`.
            std::mem::forget(manager);
            manager_ptr
        }
        Err(e) => {
            println!("Failed to create device manager: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// Returns true on success. The caller must pass in the pointer returned by
/// `OpenLib()`, and `input_remapping_ptr` must point to a UTF-8 encoded
/// `InputRemapping` message.
#[no_mangle]
pub unsafe extern "C" fn LoadInputRemapping(
    manager_ptr: *mut c_void,
    input_remapping_ptr: *const c_char,
) -> bool {
    load_input_remapping(manager_ptr, input_remapping_ptr)
        .map_err(|e| {
            println!("Failed to load input remapping: {:?}", e);
            e
        })
        .is_ok()
}

/// The caller must pass in the pointer returned by `OpenLib()`.
#[no_mangle]
pub unsafe extern "C" fn CloseLib(manager_ptr: *mut c_void) {
    println!("Closing {}", project_name());
    if !manager_ptr.is_null() {
        std::ptr::drop_in_place(manager_ptr as *mut DeviceManager);
    }
}

/// Safety: see safety comments of `LoadInputRemapping()`.
unsafe fn load_input_remapping(
    manager_ptr: *mut c_void,
    input_remapping_ptr: *const c_char,
) -> Result<()> {
    let manager = match (manager_ptr as *mut DeviceManager).as_mut() {
        Some(manager) => manager,
        None => bail!("manager_ptr is null"),
    };
    manager.load_input_remapping(input_remapping_ptr)
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

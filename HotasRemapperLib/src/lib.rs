// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

mod device_manager;
mod input_reader;
mod input_remapper;
pub(crate) mod utils;
mod virtual_device;
mod virtual_device_output;

use std::ffi::c_char;
use std::ffi::c_void;

use device_manager::DeviceManager;

#[repr(C)]
pub enum DeviceType {
    Joystick = 0,
    Throttle = 1,
    VirtualDevice = 2,
}

pub(crate) type ConnectionStatusCallback =
    unsafe extern "C" fn(DeviceType, bool);

/// The caller must call `CloseLib()` at the end with the pointer returned by
/// `OpenLib()`, and `connection_status_callback` must remain a valid function
/// pointer until then.
#[no_mangle]
pub unsafe extern "C" fn OpenLib(
    connection_status_callback: ConnectionStatusCallback,
) -> *mut c_void {
    println!("Opening {}", project_name());
    match DeviceManager::new(connection_status_callback) {
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

/// The caller must pass in the pointer returned by `OpenLib()`, and
/// `file_path_ptr` must point to a null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn LoadInputRemapping(
    manager_ptr: *mut c_void,
    file_path_ptr: *const c_char,
) {
    let file_path = match utils::new_string_from_ptr(file_path_ptr) {
        Ok(path) => path,
        Err(e) => {
            println!("Invalid input remapping file path: {}", e);
            return;
        }
    };
    match (manager_ptr as *mut DeviceManager).as_mut() {
        Some(manager) => {
            println!("Loading input remapping from {}", file_path);
            manager.load_input_remapping_from_file(&file_path.as_str());
        }
        None => println!(
            "Failed to load input remapping because manager_ptr is null!"
        ),
    }
}

/// The caller must pass in the pointer returned by `OpenLib()`.
#[no_mangle]
pub unsafe extern "C" fn CloseLib(manager_ptr: *mut c_void) {
    println!("Closing {}", project_name());
    if !manager_ptr.is_null() {
        std::ptr::drop_in_place(manager_ptr as *mut DeviceManager);
    }
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

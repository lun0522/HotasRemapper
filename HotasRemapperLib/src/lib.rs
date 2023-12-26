// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

mod device_manager;
mod hid_device;
mod hid_manager;
pub(crate) mod utils;

use std::ffi::c_void;

use hid_manager::HIDManager;

/// The caller must call `CloseLib()` at the end with the pointer returned by
/// `OpenLib()`.
#[no_mangle]
pub extern "C" fn OpenLib() -> *mut c_void {
    println!("Opening {}", project_name());
    match HIDManager::new() {
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

#[no_mangle]
pub extern "C" fn CloseLib(manager_ptr: *mut c_void) {
    println!("Closing {}", project_name());
    if !manager_ptr.is_null() {
        // We expect the caller to pass in a valid pointer to `HIDManager`.
        unsafe { std::ptr::drop_in_place(manager_ptr as *mut HIDManager) };
    }
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

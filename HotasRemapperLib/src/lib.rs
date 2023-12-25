// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

pub(crate) mod bindings;
mod hid_manager;

#[no_mangle]
pub extern "C" fn InitLib() {
    println!("Initializing {}", project_name());
    if let Err(e) = hid_manager::HIDManager::new() {
        println!("Failed to create HID manager: {:?}", e);
    }
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

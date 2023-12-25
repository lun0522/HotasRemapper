// We use camel case for the project name in Xcode convention.
#![allow(non_snake_case)]

#[no_mangle]
pub extern "C" fn PrintProjectName() {
    println!("{}", project_name());
}

pub fn project_name() -> String {
    "HOTAS Remapper".to_string()
}

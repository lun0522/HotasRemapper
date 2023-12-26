use std::ffi::c_char;

use core_foundation::base::TCFType;
use core_foundation::string::CFStringRef;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::device::IOHIDDeviceGetProperty;
use io_kit_sys::hid::keys::kIOHIDProductIDKey;
use io_kit_sys::hid::keys::kIOHIDProductKey;
use io_kit_sys::hid::keys::kIOHIDVendorIDKey;

use crate::utils::new_cf_string;
use crate::utils::new_string;

pub(crate) struct DeviceId {
    device_name: String,
    vendor_id: i32,
    product_id: i32,
}

impl DeviceId {
    pub fn from_device(device: IOHIDDeviceRef) -> Self {
        // Safe because `device` is alive, and `key` will be static strings.
        let get_property = |key: *const c_char| unsafe {
            IOHIDDeviceGetProperty(
                device,
                new_cf_string(key).unwrap().as_concrete_TypeRef(),
            )
        };
        Self {
            // Safe because the system guarantees the device name string is
            // valid.
            device_name: unsafe {
                new_string(get_property(kIOHIDProductKey) as CFStringRef)
            }
            .unwrap_or("Unknown device".to_string()),
            vendor_id: get_property(kIOHIDVendorIDKey) as i32,
            product_id: get_property(kIOHIDProductIDKey) as i32,
        }
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_fmt(format_args!(
            "[device name: {:?}, vendor id: {:#x}, product id: {:#x}]",
            self.device_name, self.vendor_id, self.product_id,
        ))
    }
}

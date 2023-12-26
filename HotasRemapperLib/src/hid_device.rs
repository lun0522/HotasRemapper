use std::ffi::c_char;
use std::ffi::c_void;

use core_foundation::base::TCFType;
use core_foundation::string::CFStringRef;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueCallback;
use io_kit_sys::hid::base::IOHIDValueRef;
use io_kit_sys::hid::device::IOHIDDeviceGetProperty;
use io_kit_sys::hid::device::IOHIDDeviceRegisterInputValueCallback;
use io_kit_sys::hid::element::IOHIDElementGetDevice;
use io_kit_sys::hid::element::IOHIDElementGetUsage;
use io_kit_sys::hid::keys::kIOHIDProductIDKey;
use io_kit_sys::hid::keys::kIOHIDProductKey;
use io_kit_sys::hid::keys::kIOHIDVendorIDKey;
use io_kit_sys::hid::value::IOHIDValueGetElement;
use io_kit_sys::hid::value::IOHIDValueGetIntegerValue;

use crate::utils::new_cf_string;
use crate::utils::new_string;

/// A trait to provide what we need for calling
/// `IOHIDDeviceRegisterInputValueCallback()`.
pub(crate) trait HandleInputValue {
    fn get_pinned_pointer(&self) -> *mut c_void;
    fn get_callback(&self) -> IOHIDValueCallback;
}

pub(crate) struct DeviceId {
    pub device_ref: IOHIDDeviceRef,
    pub device_name: String,
    pub vendor_id: u32,
    pub product_id: u32,
}

impl DeviceId {
    pub fn from_device(device_ref: IOHIDDeviceRef) -> Self {
        // Safe because `device` is alive, and `key` will be static strings.
        let get_property = |key: *const c_char| unsafe {
            IOHIDDeviceGetProperty(
                device_ref,
                new_cf_string(key).unwrap().as_concrete_TypeRef(),
            )
        };
        Self {
            device_ref,
            // Safe because the system guarantees the device name string is
            // valid.
            device_name: unsafe {
                new_string(get_property(kIOHIDProductKey) as CFStringRef)
            }
            .unwrap_or("Unknown device".to_string()),
            vendor_id: get_property(kIOHIDVendorIDKey) as u32,
            product_id: get_property(kIOHIDProductIDKey) as u32,
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

pub(crate) struct InputEvent {
    pub device_ref: IOHIDDeviceRef,
    pub usage: u32,
    pub value: i32,
}

/// A struct wrapping `IOHIDDeviceRef` from IOKit.
pub(crate) struct HIDDevice {
    device_id: DeviceId,
}

impl HIDDevice {
    /// Safety: the caller must ensure the device is alive.
    pub unsafe fn open_device(
        device_id: DeviceId,
        input_value_handler: &dyn HandleInputValue,
    ) -> Self {
        IOHIDDeviceRegisterInputValueCallback(
            device_id.device_ref,
            input_value_handler.get_callback(),
            input_value_handler.get_pinned_pointer(),
        );
        Self { device_id }
    }

    pub fn device_ref(&self) -> IOHIDDeviceRef {
        self.device_id.device_ref
    }

    pub fn read_input_event(value: IOHIDValueRef) -> Option<InputEvent> {
        // Safe because the system guarantees these references are valid.
        unsafe {
            if value.is_null() {
                return None;
            }
            let element = IOHIDValueGetElement(value);
            if element.is_null() {
                return None;
            }
            Some(InputEvent {
                device_ref: IOHIDElementGetDevice(element),
                usage: IOHIDElementGetUsage(element),
                value: IOHIDValueGetIntegerValue(value) as i32,
            })
        }
    }
}

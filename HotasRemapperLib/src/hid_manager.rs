use std::convert::From;
use std::ffi::c_void;
use std::ptr::null_mut;

use anyhow::bail;
use anyhow::Result;
use core_foundation::array::CFArray;
use core_foundation::base::kCFAllocatorDefault;
use core_foundation::base::TCFType;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::kCFRunLoopDefaultMode;
use core_foundation::runloop::CFRunLoopGetCurrent;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::keys::kIOHIDDeviceUsageKey;
use io_kit_sys::hid::keys::kIOHIDDeviceUsagePageKey;
use io_kit_sys::hid::keys::kIOHIDOptionsTypeNone;
use io_kit_sys::hid::manager::IOHIDManagerClose;
use io_kit_sys::hid::manager::IOHIDManagerCreate;
use io_kit_sys::hid::manager::IOHIDManagerOpen;
use io_kit_sys::hid::manager::IOHIDManagerRegisterDeviceMatchingCallback;
use io_kit_sys::hid::manager::IOHIDManagerRegisterDeviceRemovalCallback;
use io_kit_sys::hid::manager::IOHIDManagerScheduleWithRunLoop;
use io_kit_sys::hid::manager::IOHIDManagerSetDeviceMatchingMultiple;
use io_kit_sys::hid::usage_tables::kHIDPage_GenericDesktop;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Joystick;
use io_kit_sys::ret::kIOReturnSuccess;
use io_kit_sys::ret::IOReturn;

use crate::bindings;

pub(crate) struct HIDManager {}

impl HIDManager {
    pub fn new() -> Result<Self> {
        let matching_criteria = CFDictionary::from_CFType_pairs(&[
            (
                // Safe because the string is static.
                unsafe { bindings::new_cf_string(kIOHIDDeviceUsagePageKey)? },
                CFNumber::from(kHIDPage_GenericDesktop as i32),
            ),
            (
                // Safe because the string is static.
                unsafe { bindings::new_cf_string(kIOHIDDeviceUsageKey)? },
                CFNumber::from(kHIDUsage_GD_Joystick as i32),
            ),
        ]);
        let matching_criteria_array =
            CFArray::from_CFTypes(&[matching_criteria]);

        let manager_ref = unsafe {
            IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone)
        };
        // Safe because the system manages the lifetime of these objects.
        unsafe {
            IOHIDManagerSetDeviceMatchingMultiple(
                manager_ref,
                matching_criteria_array.as_concrete_TypeRef(),
            )
        };

        // Safe because the system manages the lifetime of the manager object.
        unsafe {
            let ret = IOHIDManagerOpen(manager_ref, kIOHIDOptionsTypeNone);
            if ret != kIOReturnSuccess {
                IOHIDManagerClose(manager_ref, kIOHIDOptionsTypeNone);
                bail!("Failed to open HID manager (error code: {})", ret);
            }

            IOHIDManagerScheduleWithRunLoop(
                manager_ref,
                CFRunLoopGetCurrent(),
                kCFRunLoopDefaultMode,
            );
            IOHIDManagerRegisterDeviceMatchingCallback(
                manager_ref,
                handle_device_matched,
                null_mut(),
            );
            IOHIDManagerRegisterDeviceRemovalCallback(
                manager_ref,
                handle_device_removed,
                null_mut(),
            );
        }

        Ok(Self {})
    }
}

extern "C" fn handle_device_matched(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _device: IOHIDDeviceRef,
) {
    println!("Device matched");
}

extern "C" fn handle_device_removed(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _device: IOHIDDeviceRef,
) {
    println!("Device removed");
}
